//! The native-code review gate (D-005, D-015, N4-01).
//!
//! All Septuvo-authored VPN and Server code is Rust (D-005). External
//! components may be written in any language, but every one is reviewed
//! before it is adopted (D-015). For a Cargo workspace such as `desktop/`,
//! this crate makes the second half mechanical: **no non-Rust native code
//! (compiled C/C++, bundled binaries, libraries linked or loaded at run time)
//! enters the build unless it is listed, with a reason, in the reviewed
//! allowlist.**
//!
//! - **`deps`**: every crate cargo would build for the target (normal, build
//!   and dev dependencies, with the features actually enabled) is inspected.
//!   It fails on native-build tooling (`cc`, `cmake`, `pkg-config`,
//!   `bindgen`…) that is not shown to be inert, on a build script that runs a
//!   compiler, on a `links` key, on linking or loading a library, and on
//!   bundled binary objects, unless the allowlist lists it; and on any crate
//!   outside the chosen N4 toolkit (GTK, Qt, Skia, web views, SDL…; N4-D1).
//! - **`artifacts`**: after a build, the build directory must contain no
//!   compiled native objects, and each binary may only need listed
//!   libraries (its ELF `DT_NEEDED` entries).
//!
//! Everything allowed is written in an allowlist file, one justified entry
//! per line (see [`Allowlist`]). Entries tied to a crate that no longer match
//! anything are errors too, so the list cannot silently grow stale. Standard
//! library only; it reads files and runs `cargo`, nothing else.

pub mod elf;
pub mod json;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Crates that exist to compile or link native (non-Rust) code.
pub const BUILD_TOOLS: &[&str] = &[
    "autotools",
    "bindgen",
    "cc",
    "cmake",
    "cxx",
    "cxx-build",
    "nasm-rs",
    "pkg-config",
    "system-deps",
    "vcpkg",
];

/// Programs that compile, assemble or link native code.
pub const COMPILERS: &[&str] = &[
    "ar", "as", "c++", "cc", "cl", "clang", "clang++", "cmake", "g++", "gcc", "ld", "make",
    "meson", "nasm", "ninja", "yasm",
];

/// Crates that bind a C/C++ application framework or bundle a large
/// non-Rust engine. Not part of the chosen N4 toolkit (N4-D1: egui/eframe;
/// GTK is permitted under D-005 but was not chosen), so the gate refuses
/// them whatever the allowlist says. Adopting one needs a new owner decision
/// and review that changes this list. Matched exactly, or as a prefix
/// followed by `-`.
pub const DENIED: &[&str] = &[
    "adw",
    "cairo",
    "cairo-rs",
    "cef",
    "cxx-qt",
    "fltk",
    "gdk",
    "gdk4",
    "gio",
    "glib",
    "gobject",
    "gtk",
    "gtk4",
    "javascriptcore",
    "libadwaita",
    "pango",
    "qmetaobject",
    "qt",
    "ritual",
    "sdl2",
    "sdl3",
    "skia",
    "skia-safe",
    "soup",
    "webkit2gtk",
    "webkit6",
    "webview",
    "wry",
];

/// Is `name` on the N4-D1 exclusion list?
#[must_use]
pub fn denied(name: &str) -> bool {
    DENIED.iter().any(|d| {
        name == *d
            || name
                .strip_prefix(d)
                .is_some_and(|rest| rest.starts_with('-'))
    })
}

// ---------------------------------------------------------------------------
// Allowlist
// ---------------------------------------------------------------------------

/// A feature condition under which a build-tool dependency is inert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Inert unless this feature is enabled.
    UnlessFeature(String),
    /// Inert only if this feature is enabled.
    IfFeature(String),
    /// Used only to look up where OS libraries are installed: it compiles
    /// and links nothing (for example `pkg-config` reading a library's
    /// `libdir`). Allowed only with a reason, and checked by the
    /// `artifacts` mode on the real build.
    QueryOnly,
}

/// One allowlist entry. `krate` is always the crate the entry is about.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)] // The fields are described on each variant.
pub enum Entry {
    /// `links <crate> <value>`: the crate's `links` key.
    Links { krate: String, value: String },
    /// `build-tool <crate> <tool> unless-feature|if-feature <feature>`: a
    /// native build tool that this crate's build script leaves unused under
    /// the condition.
    BuildTool {
        krate: String,
        tool: String,
        when: Condition,
    },
    /// `command <crate> <program>`: the build script runs this program.
    Command { krate: String, program: String },
    /// `link <crate> <name>`: the crate's sources name this library to link.
    Link { krate: String, name: String },
    /// `binary <crate> <path>`: a bundled binary file, relative to the crate.
    Binary { krate: String, path: String },
    /// `dlopen <file>`: an OS library that may be loaded at run time.
    Dlopen { file: String },
    /// `not-loaded <crate> <file>`: the crate's sources mention this library
    /// name but never load it (for example a unit-test string).
    NotLoaded { krate: String, file: String },
    /// `needed <soname>`: an OS library a binary may need at load time.
    Needed { soname: String },
}

impl Entry {
    /// True for entries about one crate (these must keep matching).
    #[must_use]
    pub fn is_crate_specific(&self) -> bool {
        !matches!(self, Entry::Dlopen { .. } | Entry::Needed { .. })
    }
}

/// The justified exceptions for one workspace.
///
/// One entry per line, `#` starts a comment line, and every entry ends with
/// ` -- ` and the reason it is an operating-system interface or inert:
///
/// ```text
/// build-tool wayland-backend cc unless-feature log -- compiles its C log shim only with `log`
/// dlopen libwayland-client.so.0 -- the Wayland display server's client library (OS)
/// needed libc.so.6 -- the system C library
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Allowlist {
    /// Entries with their line numbers and reasons.
    pub entries: Vec<(usize, Entry, String)>,
}

impl Allowlist {
    /// Parses an allowlist file.
    ///
    /// # Errors
    /// The first malformed line: unknown kind, wrong number of fields, or a
    /// missing reason.
    pub fn parse(text: &str) -> Result<Allowlist, String> {
        let mut entries = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let line_no = n + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((spec, reason)) = line.split_once(" -- ") else {
                return Err(format!(
                    "line {line_no}: every entry needs ` -- ` and a reason"
                ));
            };
            let reason = reason.trim();
            if reason.is_empty() {
                return Err(format!("line {line_no}: the reason is empty"));
            }
            let f: Vec<&str> = spec.split_whitespace().collect();
            let s = |i: usize| f[i].to_string();
            let entry = match (f.first().copied(), f.len()) {
                (Some("links"), 3) => Entry::Links {
                    krate: s(1),
                    value: s(2),
                },
                (Some("build-tool"), 4 | 5) => {
                    let when = match (f[3], f.len()) {
                        ("unless-feature", 5) => Condition::UnlessFeature(s(4)),
                        ("if-feature", 5) => Condition::IfFeature(s(4)),
                        ("query-only", 4) => Condition::QueryOnly,
                        _ => {
                            return Err(format!(
                                "line {line_no}: expected unless-feature FEATURE, if-feature FEATURE or query-only"
                            ));
                        }
                    };
                    Entry::BuildTool {
                        krate: s(1),
                        tool: s(2),
                        when,
                    }
                }
                (Some("command"), 3) => Entry::Command {
                    krate: s(1),
                    program: s(2),
                },
                (Some("link"), 3) => Entry::Link {
                    krate: s(1),
                    name: s(2),
                },
                (Some("binary"), 3) => Entry::Binary {
                    krate: s(1),
                    path: s(2),
                },
                (Some("dlopen"), 2) => Entry::Dlopen { file: s(1) },
                (Some("not-loaded"), 3) => Entry::NotLoaded {
                    krate: s(1),
                    file: s(2),
                },
                (Some("needed"), 2) => Entry::Needed { soname: s(1) },
                (Some(kind), _) => {
                    return Err(format!(
                        "line {line_no}: unknown entry `{kind}` or wrong number of fields"
                    ));
                }
                (None, _) => return Err(format!("line {line_no}: empty entry")),
            };
            if let Entry::BuildTool { tool, .. } = &entry
                && !BUILD_TOOLS.contains(&tool.as_str())
            {
                return Err(format!(
                    "line {line_no}: `{tool}` is not a known build tool"
                ));
            }
            entries.push((line_no, entry, reason.to_string()));
        }
        Ok(Allowlist { entries })
    }

    fn find(&self, used: &mut BTreeSet<usize>, pred: impl Fn(&Entry) -> bool) -> Option<&Entry> {
        let (i, (_, e, _)) = self
            .entries
            .iter()
            .enumerate()
            .find(|(_, (_, e, _))| pred(e))?;
        used.insert(i);
        Some(e)
    }
}

// ---------------------------------------------------------------------------
// The packages cargo builds
// ---------------------------------------------------------------------------

/// One crate as cargo would build it for the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Crate name.
    pub name: String,
    /// Version.
    pub version: String,
    /// Directory holding its `Cargo.toml`.
    pub dir: PathBuf,
    /// Its `links` key, if any.
    pub links: Option<String>,
    /// Its build script, if any.
    pub build_script: Option<PathBuf>,
    /// Its declared build dependencies: name, and whether optional.
    pub build_deps: Vec<(String, bool)>,
    /// Features enabled in this build.
    pub features: Vec<String>,
}

/// Parses `cargo tree --prefix none --format '{p}|{f}'` output into
/// `(name, version, features)`, without duplicates.
///
/// # Errors
/// A line that is not in that format.
pub fn parse_tree(text: &str) -> Result<Vec<(String, String, Vec<String>)>, String> {
    let mut seen = BTreeMap::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (p, f) = line
            .split_once('|')
            .ok_or_else(|| format!("unexpected cargo tree line: {line}"))?;
        let mut parts = p.split_whitespace();
        let (Some(name), Some(v)) = (parts.next(), parts.next()) else {
            return Err(format!("unexpected cargo tree line: {line}"));
        };
        let Some(version) = v.strip_prefix('v') else {
            return Err(format!("unexpected cargo tree line: {line}"));
        };
        let features = f
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        seen.entry((name.to_string(), version.to_string()))
            .or_insert(features);
    }
    Ok(seen.into_iter().map(|((n, v), f)| (n, v, f)).collect())
}

/// Builds the package list from `cargo metadata` JSON (for locations, `links`,
/// build scripts and declared dependencies) and the `cargo tree` set (which
/// crates are actually built, with which features).
///
/// # Errors
/// Malformed input, or a built crate missing from the metadata.
pub fn packages_from(metadata: &str, tree: &str) -> Result<Vec<Package>, String> {
    let meta = json::parse(metadata).map_err(|e| format!("cargo metadata: {e}"))?;
    let mut by_key = BTreeMap::new();
    for p in meta
        .get("packages")
        .map(json::Value::items)
        .unwrap_or_default()
    {
        let name = p
            .get("name")
            .and_then(json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let version = p
            .get("version")
            .and_then(json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let manifest = p
            .get("manifest_path")
            .and_then(json::Value::as_str)
            .unwrap_or_default();
        let dir = Path::new(manifest)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let links = p
            .get("links")
            .and_then(json::Value::as_str)
            .map(String::from);
        let build_script = p
            .get("targets")
            .map(json::Value::items)
            .unwrap_or_default()
            .iter()
            .find(|t| {
                t.get("kind")
                    .map(json::Value::items)
                    .unwrap_or_default()
                    .iter()
                    .any(|k| k.as_str() == Some("custom-build"))
            })
            .and_then(|t| t.get("src_path").and_then(json::Value::as_str))
            .map(PathBuf::from);
        let build_deps = p
            .get("dependencies")
            .map(json::Value::items)
            .unwrap_or_default()
            .iter()
            .filter(|d| d.get("kind").and_then(json::Value::as_str) == Some("build"))
            .filter_map(|d| {
                let name = d.get("name").and_then(json::Value::as_str)?.to_string();
                Some((name, d.get("optional") == Some(&json::Value::Bool(true))))
            })
            .collect();
        by_key.insert(
            (name.clone(), version.clone()),
            Package {
                name,
                version,
                dir,
                links,
                build_script,
                build_deps,
                features: Vec::new(),
            },
        );
    }
    let mut out = Vec::new();
    for (name, version, features) in parse_tree(tree)? {
        let mut p = by_key
            .get(&(name.clone(), version.clone()))
            .cloned()
            .ok_or_else(|| format!("{name} {version} is built but missing from cargo metadata"))?;
        p.features = features;
        out.push(p);
    }
    Ok(out)
}

/// Runs `cargo metadata` and `cargo tree` (both `--locked`) for a workspace
/// and returns the crates built for `target`.
///
/// # Errors
/// cargo fails or prints something unexpected.
pub fn load(manifest: &Path, target: &str) -> Result<Vec<Package>, String> {
    let run = |args: &[&str]| -> Result<String, String> {
        let out = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(args)
            .arg("--manifest-path")
            .arg(manifest)
            .output()
            .map_err(|e| format!("cannot run cargo: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "cargo {} failed: {}",
                args[0],
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        String::from_utf8(out.stdout).map_err(|_| "cargo printed invalid UTF-8".to_string())
    };
    let metadata = run(&[
        "metadata",
        "--locked",
        "--format-version",
        "1",
        "--filter-platform",
        target,
    ])?;
    let tree = run(&[
        "tree",
        "--locked",
        "--target",
        target,
        "-e",
        "normal,build,dev",
        "--prefix",
        "none",
        "--no-dedupe",
        "--format",
        "{p}|{f}",
    ])?;
    packages_from(&metadata, &tree)
}

// ---------------------------------------------------------------------------
// Scanning one crate's files
// ---------------------------------------------------------------------------

/// What one crate's files show.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scan {
    /// Programs its build script runs that compile or link native code.
    pub commands: BTreeSet<String>,
    /// Libraries named for linking (`#[link(name)]`, `rustc-link-lib`).
    pub links: BTreeSet<String>,
    /// Shared libraries named in string literals (loaded at run time).
    pub dlopen: BTreeSet<String>,
    /// Bundled binary files (relative paths).
    pub binaries: BTreeSet<String>,
    /// C, C++, Objective-C or assembly files (relative paths).
    pub native_sources: BTreeSet<String>,
}

const MAX_FILES: usize = 100_000;
const MAX_READ: u64 = 8 * 1024 * 1024;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = fs::read_dir(&d).map_err(|e| format!("cannot read {}: {e}", d.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("cannot read {}: {e}", d.display()))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if kind.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    stack.push(path);
                }
            } else if kind.is_file() {
                out.push(path);
                if out.len() > MAX_FILES {
                    return Err(format!("{}: too many files", dir.display()));
                }
            }
        }
    }
    out.sort();
    Ok(())
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn is_binary_object(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    matches!(
        extension(path).as_str(),
        "a" | "o" | "obj" | "lib" | "dll" | "dylib" | "so" | "exe"
    ) || name.contains(".so.")
}

fn is_native_source(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "c" | "cc" | "cpp" | "cxx" | "m" | "mm" | "s" | "S" | "asm"
    )
}

fn read_text(path: &Path) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.len() > MAX_READ {
        return Err(format!("{}: too large to inspect", path.display()));
    }
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Removes `//` comment lines (including doc comments), whose examples
/// would otherwise look like real library names.
fn code_lines(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `"…"` string literal's contents in `text` (escapes not decoded).
fn string_literals(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j] != b'"' {
                if b[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            if j <= b.len() {
                out.push(&text[start..j.min(b.len())]);
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// A shared-library file name: `lib<name>.so` optionally followed by
/// `.<digits>` parts.
fn shared_library(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("lib") else {
        return false;
    };
    let Some((stem, tail)) = rest.split_once(".so") else {
        return false;
    };
    !stem.is_empty()
        && stem
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'+'))
        && (tail.is_empty()
            || tail.strip_prefix('.').is_some_and(|t| {
                t.split('.')
                    .all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()))
            }))
}

/// Library names in `#[link(name = "…")]` attributes.
fn link_attributes(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = code;
    while let Some(at) = rest.find("#[link(") {
        rest = &rest[at + 7..];
        let attr = &rest[..rest.find(')').unwrap_or(rest.len())];
        if let Some(n) = attr.find("name") {
            let after = attr[n + 4..].trim_start();
            if let Some(v) = after.strip_prefix('=') {
                let v = v.trim_start();
                if let Some(v) = v.strip_prefix('"') {
                    out.push(v[..v.find('"').unwrap_or(v.len())].to_string());
                }
            }
        }
    }
    out
}

/// Library names a build script asks rustc to link (`rustc-link-lib=`).
fn link_directives(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (at, _) in code.match_indices("rustc-link-lib=") {
        let v = &code[at + "rustc-link-lib=".len()..];
        let end = v
            .find(|c: char| c == '"' || c == '\'' || c.is_whitespace() || c == '{')
            .unwrap_or(v.len());
        let mut name = &v[..end];
        for kind in ["dylib=", "static=", "framework="] {
            name = name.strip_prefix(kind).unwrap_or(name);
        }
        out.push(if name.is_empty() {
            "(computed)".to_string()
        } else {
            name.to_string()
        });
    }
    out
}

/// Programs started with `Command::new("…")`.
fn commands(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (at, _) in code.match_indices("Command::new(") {
        let v = code[at + 13..].trim_start();
        if let Some(v) = v.strip_prefix('"') {
            out.push(v[..v.find('"').unwrap_or(v.len())].to_string());
        }
    }
    out
}

/// Inspects one crate's files.
///
/// # Errors
/// The crate's directory cannot be read.
pub fn scan(p: &Package) -> Result<Scan, String> {
    let mut files = Vec::new();
    walk(&p.dir, &mut files)?;
    let mut s = Scan::default();
    let rel = |f: &Path| {
        f.strip_prefix(&p.dir)
            .unwrap_or(f)
            .to_string_lossy()
            .into_owned()
    };
    let src = p.dir.join("src");
    for f in &files {
        if is_binary_object(f) {
            s.binaries.insert(rel(f));
        }
        if is_native_source(f) {
            s.native_sources.insert(rel(f));
        }
        let is_build = p.build_script.as_deref() == Some(f.as_path());
        if extension(f) == "rs" && (f.starts_with(&src) || is_build) {
            let code = code_lines(&read_text(f)?);
            s.links.extend(link_attributes(&code));
            for lit in string_literals(&code) {
                if shared_library(lit) {
                    s.dlopen.insert(lit.to_string());
                }
            }
            if is_build {
                s.links.extend(link_directives(&code));
                s.commands.extend(
                    commands(&code)
                        .into_iter()
                        .filter(|c| COMPILERS.contains(&c.as_str())),
                );
            }
        }
    }
    // A build script outside src/ (the usual build.rs) was read above only if
    // it is in the file list; make sure it is read even if it lives elsewhere.
    if let Some(b) = &p.build_script
        && !files.contains(b)
    {
        let code = code_lines(&read_text(b)?);
        s.links.extend(link_directives(&code));
        s.commands.extend(
            commands(&code)
                .into_iter()
                .filter(|c| COMPILERS.contains(&c.as_str())),
        );
    }
    Ok(s)
}

// ---------------------------------------------------------------------------
// The checks
// ---------------------------------------------------------------------------

/// The outcome of a check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Crates inspected.
    pub crates: usize,
    /// Everything that breaks the rule. Empty means pass.
    pub violations: Vec<String>,
    /// OS libraries the crates may load or need (allowed).
    pub os_interfaces: BTreeSet<String>,
    /// Inert native files: present in a crate but nothing can compile them.
    pub inert_native: Vec<String>,
    /// Explanations of allowlisted findings, for the log.
    pub allowed: Vec<String>,
}

impl Report {
    /// True if nothing breaks the rule.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

fn reason(list: &Allowlist, e: &Entry) -> String {
    list.entries
        .iter()
        .find(|(_, x, _)| x == e)
        .map(|(_, _, r)| r.clone())
        .unwrap_or_default()
}

/// The checks that depend on a crate's files.
fn check_scan(
    id: &str,
    p: &Package,
    s: &Scan,
    list: &Allowlist,
    used: &mut BTreeSet<usize>,
    r: &mut Report,
) {
    for c in &s.commands {
        match list.find(
            used,
            |e| matches!(e, Entry::Command { krate, program } if *krate == p.name && program == c),
        ) {
            Some(e) => r
                .allowed
                .push(format!("{id}: build script runs {c}: {}", reason(list, e))),
            None => r.violations.push(format!(
                "{id}: build script runs `{c}` (native compiler or linker)"
            )),
        }
    }
    for l in &s.links {
        match list.find(
            used,
            |e| matches!(e, Entry::Link { krate, name } if *krate == p.name && name == l),
        ) {
            Some(e) => r
                .allowed
                .push(format!("{id}: names library {l}: {}", reason(list, e))),
            None => r.violations.push(format!(
                "{id}: links library `{l}`, which is not in the reviewed allowlist"
            )),
        }
    }
    for b in &s.binaries {
        match list.find(
            used,
            |e| matches!(e, Entry::Binary { krate, path } if *krate == p.name && path == b),
        ) {
            Some(e) => r
                .allowed
                .push(format!("{id}: contains {b}: {}", reason(list, e))),
            None => r
                .violations
                .push(format!("{id}: bundles the binary file `{b}`")),
        }
    }
    for d in &s.dlopen {
        if let Some(e) = list.find(
            used,
            |e| matches!(e, Entry::NotLoaded { krate, file } if *krate == p.name && file == d),
        ) {
            r.allowed.push(format!(
                "{id}: mentions {d} without loading it: {}",
                reason(list, e)
            ));
        } else if list
            .find(used, |e| matches!(e, Entry::Dlopen { file } if file == d))
            .is_some()
        {
            r.os_interfaces.insert(d.clone());
        } else {
            r.violations.push(format!(
                "{id}: may load `{d}` at run time, which is not in the reviewed allowlist"
            ));
        }
    }
    if !s.native_sources.is_empty() {
        r.inert_native.push(format!(
            "{id}: {} native source file(s), not compiled (no compiler in this build): {}",
            s.native_sources.len(),
            s.native_sources
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
}

/// Checks the crates of one workspace against the allowlist.
#[must_use]
pub fn check_packages(packages: &[Package], scans: &[Scan], list: &Allowlist) -> Report {
    let mut r = Report {
        crates: packages.len(),
        ..Report::default()
    };
    let mut used = BTreeSet::new();
    let built: BTreeSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let mut tools_explained: BTreeSet<&str> = BTreeSet::new();

    for (p, s) in packages.iter().zip(scans) {
        let id = format!("{} {}", p.name, p.version);
        if denied(&p.name) {
            r.violations.push(format!(
                "{id}: binds a C/C++ application framework or bundles a large non-Rust engine (excluded by the N4 toolkit decision, N4-D1)"
            ));
        }
        if let Some(value) = &p.links {
            match list.find(
                &mut used,
                |e| matches!(e, Entry::Links { krate, value: v } if *krate == p.name && v == value),
            ) {
                Some(e) => r
                    .allowed
                    .push(format!("{id}: links = {value}: {}", reason(list, e))),
                None => r.violations.push(format!(
                    "{id}: declares `links = \"{value}\"` (links a native library)"
                )),
            }
        }
        // An optional dependency is part of this build only if its feature is on.
        let active = |(name, optional): &(String, bool)| {
            !optional
                || p.features
                    .iter()
                    .any(|f| f == name || f.strip_prefix("dep:") == Some(name))
        };
        for (tool, _) in p.build_deps.iter().filter(|d| {
            BUILD_TOOLS.contains(&d.0.as_str()) && built.contains(d.0.as_str()) && active(d)
        }) {
            let entry = list.find(&mut used, |e| matches!(e, Entry::BuildTool { krate, tool: t, .. } if *krate == p.name && t == tool));
            match entry {
                None => r.violations.push(format!(
                    "{id}: build dependency `{tool}` compiles or links native code"
                )),
                Some(e @ Entry::BuildTool { when, .. }) => {
                    let on = |f: &str| p.features.iter().any(|x| x == f);
                    let inert = match when {
                        Condition::UnlessFeature(f) => !on(f),
                        Condition::IfFeature(f) => on(f),
                        Condition::QueryOnly => true,
                    };
                    if inert {
                        tools_explained.insert(tool);
                        r.allowed.push(format!(
                            "{id}: build dependency {tool} is inert here: {}",
                            reason(list, e)
                        ));
                    } else {
                        r.violations.push(format!(
                            "{id}: build dependency `{tool}` is active with the enabled features ({})",
                            p.features.join(",")
                        ));
                    }
                }
                Some(_) => {}
            }
        }
        check_scan(&id, p, s, list, &mut used, &mut r);
    }
    for tool in BUILD_TOOLS
        .iter()
        .filter(|t| built.contains(*t) && !tools_explained.contains(*t))
    {
        if !r
            .violations
            .iter()
            .any(|v| v.contains(&format!("`{tool}`")))
        {
            r.violations.push(format!(
                "native build tool `{tool}` is built, and no crate shows it to be inert"
            ));
        }
    }
    for (i, (line, e, _)) in list.entries.iter().enumerate() {
        if e.is_crate_specific() && !used.contains(&i) {
            r.violations.push(format!(
                "allowlist line {line}: matches nothing; remove it ({e:?})"
            ));
        }
    }
    r
}

/// Loads, scans and checks a workspace.
///
/// # Errors
/// cargo or the file system fails.
pub fn check_workspace(manifest: &Path, target: &str, list: &Allowlist) -> Result<Report, String> {
    let packages = load(manifest, target)?;
    let scans = packages.iter().map(scan).collect::<Result<Vec<_>, _>>()?;
    Ok(check_packages(&packages, &scans, list))
}

/// Native objects produced by build scripts under a cargo target directory
/// (`…/build/<crate>-<hash>/out/`). There must be none.
///
/// # Errors
/// The directory cannot be read.
pub fn native_build_outputs(target_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut stack = vec![target_dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() => stack.push(path),
                Ok(t) if t.is_file() => {
                    let in_out = path.components().any(|c| c.as_os_str() == "out")
                        && path.components().any(|c| c.as_os_str() == "build");
                    if in_out && is_binary_object(&path) {
                        files.push(path);
                    }
                }
                _ => {}
            }
        }
    }
    if !target_dir.is_dir() {
        return Err(format!("{}: not a directory", target_dir.display()));
    }
    files.sort();
    Ok(files)
}

/// Checks built artifacts: no native objects from build scripts, and each
/// binary needs only listed OS libraries.
///
/// # Errors
/// A file cannot be read or is not an ELF binary.
pub fn check_artifacts(
    target_dir: &Path,
    binaries: &[PathBuf],
    list: &Allowlist,
) -> Result<Report, String> {
    let mut r = Report {
        crates: binaries.len(),
        ..Report::default()
    };
    for f in native_build_outputs(target_dir)? {
        r.violations.push(format!(
            "a build script produced the native object {}",
            f.display()
        ));
    }
    let mut used = BTreeSet::new();
    for b in binaries {
        let bytes = fs::read(b).map_err(|e| format!("{}: {e}", b.display()))?;
        let needed = elf::needed(&bytes).map_err(|e| format!("{}: {e}", b.display()))?;
        for lib in needed {
            if list
                .find(
                    &mut used,
                    |e| matches!(e, Entry::Needed { soname } if *soname == lib),
                )
                .is_some()
            {
                r.os_interfaces.insert(lib);
            } else {
                r.violations.push(format!(
                    "{} needs `{lib}`, which is not in the reviewed allowlist",
                    b.display()
                ));
            }
        }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str) -> Package {
        Package {
            name: name.into(),
            version: "1.0.0".into(),
            dir: PathBuf::new(),
            links: None,
            build_script: None,
            build_deps: Vec::new(),
            features: Vec::new(),
        }
    }

    #[test]
    fn allowlist_needs_reasons_and_known_kinds() {
        assert!(Allowlist::parse("dlopen libX11.so.6").is_err());
        assert!(Allowlist::parse("dlopen libX11.so.6 -- ").is_err());
        assert!(Allowlist::parse("frobnicate a b -- why").is_err());
        assert!(Allowlist::parse("build-tool a gcc unless-feature x -- why").is_err());
        assert!(Allowlist::parse("build-tool a cc sometimes x -- why").is_err());
        let l = Allowlist::parse("# c\n\nneeded libc.so.6 -- the C library\nbuild-tool w cc unless-feature log -- shim\n").unwrap();
        assert_eq!(l.entries.len(), 2);
        assert_eq!(l.entries[1].0, 4);
    }

    #[test]
    fn deny_list_matches_names_and_families() {
        for n in [
            "gtk4",
            "gtk",
            "gtk4-sys",
            "glib-sys",
            "libadwaita",
            "skia-safe",
            "webkit2gtk",
            "wry",
            "qt-core",
        ] {
            assert!(denied(n), "{n}");
        }
        for n in ["glow", "gimli", "egui", "glutin", "qtest", "cairo2x"] {
            assert!(!denied(n), "{n}");
        }
    }

    #[test]
    fn build_tools_need_a_true_inert_condition() {
        let tools = |features: &[&str]| {
            let mut w = pkg("wayland-backend");
            w.build_deps = vec![("cc".into(), false)];
            w.features = features.iter().map(|s| (*s).to_string()).collect();
            vec![w, pkg("cc")]
        };
        let none = [Scan::default(), Scan::default()];
        let list = Allowlist::parse(
            "build-tool wayland-backend cc unless-feature log -- C shim only with log",
        )
        .unwrap();
        assert!(check_packages(&tools(&["dlopen"]), &none, &list).passed());
        let r = check_packages(&tools(&["log"]), &none, &list);
        assert!(!r.passed());
        assert!(r.violations[0].contains("active"), "{:?}", r.violations);
        let r = check_packages(&tools(&[]), &none, &Allowlist::default());
        assert!(
            r.violations
                .iter()
                .any(|v| v.contains("build dependency `cc`")),
            "{:?}",
            r.violations
        );
        // An optional build tool whose feature is off is not part of the build.
        let mut s = pkg("sctk");
        s.build_deps = vec![("pkg-config".into(), true)];
        let r = check_packages(
            &[s.clone(), pkg("pkg-config")],
            &none,
            &Allowlist::default(),
        );
        assert!(
            r.violations.iter().all(|v| !v.starts_with("sctk")),
            "{:?}",
            r.violations
        );
        s.features = vec!["pkg-config".into()];
        let r = check_packages(&[s, pkg("pkg-config")], &none, &Allowlist::default());
        assert!(
            r.violations.iter().any(|v| v.starts_with("sctk")),
            "{:?}",
            r.violations
        );
        // A query-only tool is accepted with its reason.
        let mut x = pkg("x11-dl");
        x.build_deps = vec![("pkg-config".into(), false)];
        let list = Allowlist::parse("build-tool x11-dl pkg-config query-only -- reads libdir only")
            .unwrap();
        assert!(check_packages(&[x, pkg("pkg-config")], &none, &list).passed());
        assert!(Allowlist::parse("build-tool x pkg-config query-only extra -- r").is_err());
        // A tool that is built but declared by no crate is still caught.
        let r = check_packages(&[pkg("cmake")], &[Scan::default()], &Allowlist::default());
        assert!(!r.passed());
    }

    #[test]
    fn every_finding_needs_an_entry_and_stale_entries_fail() {
        let mut p = pkg("x");
        p.links = Some("foo".into());
        let s = Scan {
            commands: ["gcc".to_string()].into(),
            links: ["foo".to_string()].into(),
            dlopen: ["libfoo.so.1".to_string()].into(),
            binaries: ["vendor/libfoo.a".to_string()].into(),
            native_sources: ["foo.c".to_string()].into(),
        };
        let r = check_packages(
            std::slice::from_ref(&p),
            std::slice::from_ref(&s),
            &Allowlist::default(),
        );
        assert_eq!(r.violations.len(), 5, "{:?}", r.violations);
        assert_eq!(r.inert_native.len(), 1);
        let list = Allowlist::parse(
            "links x foo -- a\ncommand x gcc -- b\nlink x foo -- c\nbinary x vendor/libfoo.a -- d\ndlopen libfoo.so.1 -- e\n",
        )
        .unwrap();
        assert!(check_packages(std::slice::from_ref(&p), std::slice::from_ref(&s), &list).passed());
        let stale = Allowlist::parse(
            "link y bar -- gone\ndlopen libunused.so -- global lists may be wider\n",
        )
        .unwrap();
        let r = check_packages(&[pkg("z")], &[Scan::default()], &stale);
        assert_eq!(r.violations.len(), 1, "{:?}", r.violations);
        assert!(r.violations[0].contains("line 1"));
    }

    #[test]
    fn text_scanners_find_what_matters_and_skip_comments() {
        let code = code_lines(
            "/// dlopen(\"libdoc.so\")\n#[link(name = \"xcb\")]\nextern {}\nlet l = \"libwayland-client.so.0\";\n\
             // Command::new(\"gcc\")\nCommand::new(\"cc\").arg(\"x.c\");\nprintln!(\"cargo:rustc-link-lib=dylib=dl\");\n",
        );
        assert_eq!(link_attributes(&code), vec!["xcb"]);
        assert_eq!(link_directives(&code), vec!["dl"]);
        assert_eq!(commands(&code), vec!["cc"]);
        let libs: Vec<_> = string_literals(&code)
            .into_iter()
            .filter(|l| shared_library(l))
            .collect();
        assert_eq!(libs, vec!["libwayland-client.so.0"]);
        for s in ["libGL.so.1", "libxkbcommon-x11.so.0", "libX11.so"] {
            assert!(shared_library(s), "{s}");
        }
        for s in [
            "lib.so",
            "libfoo.so.x",
            "libfoo.dylib",
            "foo.so",
            "libfoo.so.",
        ] {
            assert!(!shared_library(s), "{s}");
        }
    }

    #[test]
    fn tree_lines_parse_with_and_without_paths() {
        let t = "eframe v0.36.2|accesskit,glow\nseptuvo-cli v0.0.0 (/x/crates/septuvo-cli)|\neframe v0.36.2|accesskit,glow\n";
        let p = parse_tree(t).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].0, "eframe");
        assert_eq!(p[0].2, vec!["accesskit", "glow"]);
        assert!(parse_tree("nonsense\n").is_err());
        assert!(parse_tree("a 1.0|x\n").is_err());
    }
}
