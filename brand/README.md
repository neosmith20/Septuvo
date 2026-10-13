# Septuvo — approved visual brand kit (v1)

**Owner approved the visual direction and 81-file review archive on 2026-10-13.** The primary mark is a dimensional metallic blue/cyan/silver ribbon S with an outlined wordmark and the tagline **Security Without Compromise.**

## Product family

- **Septuvo:** primary company/brand
- **Septuvo VPN:** consumer and business VPN software (blue)
- **Septuvo Protocol (SVP):** independent networking protocol (teal)
- **Septuvo Server:** self-hosted/commercial gateways (purple)

## Source priority

1. The original approved concept board inside `Septuvo_Full_Brand_Kit_Review_v1.zip` is the **authoritative visual appearance**.
2. Transparent PNG logos and app icon exports in that ZIP preserve the actual cut-out raster emblem.
3. The SVGs under [vectors](vectors/) are useful scalable **interpretations**, **not pixel-identical tracings** of the metallic raster. Do not replace production assets without visual approval.
4. The canonical editable palette is in [septuvo-colors.json](tokens/septuvo-colors.json); use the accompanying CSS and Rust tokens.

**Archive integrity**: `Septuvo_Full_Brand_Kit_Review_v1.zip` = SHA-256 `57f3ccc4daac9ef64a38e8331aac90c5147af2dcfaba31ac51ff915d9eee8e0d`, 81 files, ~16 MB. The full binary archive must be separately uploaded and verified. Its absence must **not** be represented as a completed repository transfer.

## Asset and typography rules

- Keep geometry and aspect ratio intact, and allow clear space.
- Prefer full-color mark on dark surfaces; use mono navy on white backgrounds and white mark on dark backgrounds.
- Test app icons at 16, 24, 32, 64, 256 and 1024 pixels (the tiny icons are actual distinct exports).
- Primary typeface: **Inter** (Regular / Medium / SemiBold / Bold). No font binaries are included; use its official license and distribution.
- Never signify authenticated tunnel status with color alone; UI must differentiate adapter, handshake, packet reachability and DNS.
- Do not infer production cryptographic guarantees or trademark clearance from the name/tagline/brand materials.

## Archive completion checklist

- [ ] Upload exact 81-file review ZIP as `brand/package/Septuvo_Full_Brand_Kit_Review_v1.zip`.
- [ ] Compare SHA-256 to value above.
- [ ] Verify original approved board, nine-page PDF and all icon exports are present.
- [ ] Recheck vector/logo match before using the approximations as a definitive trademark master.

The repository brand sources are for future implementation; they do not alter the project security gates.
