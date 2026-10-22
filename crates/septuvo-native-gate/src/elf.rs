//! Reading the libraries a Linux binary needs at load time (`DT_NEEDED`).
//!
//! The same information `readelf -d` prints, from a 64-bit little-endian ELF
//! file. Every offset is bounds-checked: a truncated or malformed file is an
//! error, never a panic.

/// Why a file could not be read as a 64-bit little-endian ELF binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub &'static str);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a readable 64-bit ELF binary: {}", self.0)
    }
}

impl std::error::Error for Error {}

fn u16_at(b: &[u8], at: usize) -> Result<u16, Error> {
    let s = b.get(at..at + 2).ok_or(Error("truncated"))?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

fn u64_at(b: &[u8], at: usize) -> Result<u64, Error> {
    let s = b.get(at..at + 8).ok_or(Error("truncated"))?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Ok(u64::from_le_bytes(a))
}

fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    let s = b.get(at..at + 4).ok_or(Error("truncated"))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn to_usize(v: u64) -> Result<usize, Error> {
    usize::try_from(v).map_err(|_| Error("offset out of range"))
}

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;

/// The `DT_NEEDED` entries of an ELF binary, in order.
///
/// # Errors
/// The file is not a 64-bit little-endian ELF file, or is malformed.
pub fn needed(b: &[u8]) -> Result<Vec<String>, Error> {
    if b.get(0..4) != Some(b"\x7fELF") {
        return Err(Error("bad magic"));
    }
    if b.get(4) != Some(&2) || b.get(5) != Some(&1) {
        return Err(Error("only 64-bit little-endian files are supported"));
    }
    let phoff = to_usize(u64_at(b, 0x20)?)?;
    let phentsize = usize::from(u16_at(b, 0x36)?);
    let phnum = usize::from(u16_at(b, 0x38)?);
    if phentsize < 56 {
        return Err(Error("bad program header size"));
    }
    let mut loads = Vec::new();
    let mut dynamic = None;
    for k in 0..phnum {
        let h = phoff
            .checked_add(k.checked_mul(phentsize).ok_or(Error("overflow"))?)
            .ok_or(Error("overflow"))?;
        let kind = u32_at(b, h)?;
        let offset = u64_at(b, h + 8)?;
        let vaddr = u64_at(b, h + 16)?;
        let filesz = u64_at(b, h + 32)?;
        match kind {
            PT_LOAD => loads.push((vaddr, offset, filesz)),
            PT_DYNAMIC => dynamic = Some((offset, filesz)),
            _ => {}
        }
    }
    // A statically linked binary needs nothing.
    let Some((dyn_off, dyn_size)) = dynamic else {
        return Ok(Vec::new());
    };
    let (dyn_off, dyn_size) = (to_usize(dyn_off)?, to_usize(dyn_size)?);
    let mut strtab = None;
    let mut wanted = Vec::new();
    let mut at = dyn_off;
    while at + 16 <= dyn_off.saturating_add(dyn_size) {
        let tag = u64_at(b, at)?;
        let val = u64_at(b, at + 8)?;
        match tag {
            DT_NULL => break,
            DT_NEEDED => wanted.push(val),
            DT_STRTAB => strtab = Some(val),
            _ => {}
        }
        at += 16;
    }
    let strtab = strtab.ok_or(Error("no string table"))?;
    // The string table is given as a virtual address: map it to a file offset.
    let base = loads
        .iter()
        .find(|&&(vaddr, _, filesz)| strtab >= vaddr && strtab - vaddr < filesz)
        .map(|&(vaddr, offset, _)| strtab - vaddr + offset)
        .ok_or(Error("string table outside the file"))?;
    let mut out = Vec::new();
    for off in wanted {
        let start = to_usize(base.checked_add(off).ok_or(Error("overflow"))?)?;
        let rest = b.get(start..).ok_or(Error("name outside the file"))?;
        let end = rest
            .iter()
            .position(|&c| c == 0)
            .ok_or(Error("unterminated name"))?;
        let name = std::str::from_utf8(&rest[..end]).map_err(|_| Error("name is not UTF-8"))?;
        out.push(name.to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_elf_and_truncated_files() {
        assert!(needed(b"").is_err());
        assert!(needed(b"#!/bin/sh\n").is_err());
        assert!(needed(b"\x7fELF\x02\x01\x01").is_err());
        assert!(needed(b"\x7fELF\x01\x01\x01").is_err());
    }

    #[test]
    fn reads_the_test_binary_itself() {
        // The running test binary is a dynamically linked Linux executable
        // that needs at least the C library.
        let me = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let libs = needed(&me).unwrap();
        assert!(libs.iter().any(|l| l.starts_with("libc.so")), "{libs:?}");
    }
}
