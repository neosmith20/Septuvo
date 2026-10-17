# Dependency review: cargo-fuzz and libfuzzer-sys (H-018, D-035)

**The D-015 dependency-review record.** Prepared 2026-10-17 for the owner's request to add coverage-guided fuzzing ("add in the advanced kind of fuzzing please (cargo-fuzz)", 2026-10-17). It is accepted when the owner merges the pull request that adds it (D-031).

## Scope: development only

**The scope is enforced in three ways:**
- **Separate workspace.** The dependencies live only in `fuzz/`, a separate Cargo workspace (excluded from the main one). The main workspace keeps **no third-party crates**: its `Cargo.lock` is still checked in CI, and none of Septuvo's crates depends on anything here.
- **Never shipped.** Nothing here is built into, linked with or shipped with Septuvo VPN, Septuvo Server or any release. The fuzz binaries exist only on developer machines and CI runners.
- **No cryptography.** None of these crates is cryptographic, and none adds cryptography to Septuvo. The G1 boundary is unaffected.

## What is added

| Component | Version (exact pin) | Published | License | Role |
| --- | --- | --- | --- | --- |
| `libfuzzer-sys` | 0.4.13 | 2026-06-04 | (MIT OR Apache-2.0) AND NCSA | Rust bindings for libFuzzer; **bundles LLVM libFuzzer (C++)**, compiled with the system C++ compiler |
| `arbitrary` | 1.4.2 | 2025-08-14 | MIT OR Apache-2.0 | Dependency of `libfuzzer-sys` (not used by our targets) |
| `cc` | 1.5.1 | 2026-09-25 | MIT OR Apache-2.0 | Build-time: compiles libFuzzer |
| `find-msvc-tools`, `jobserver`, `shlex`, `libc`, `getrandom`, `cfg-if`, `r-efi` | see `fuzz/Cargo.lock` | 2026-02 to 2026-10-02 | MIT OR Apache-2.0 (`r-efi` also LGPL-2.1-or-later as a choice) | Transitive, build-time helpers of `cc` |
| `cargo-fuzz` (the tool) | 0.13.2 | 2026-06-09 | MIT OR Apache-2.0 | Installed in CI with `cargo install --locked`; not a dependency of any crate |
| Rust nightly | `nightly-2026-10-01` | — | MIT OR Apache-2.0 | Needed for the coverage and sanitizer flags cargo-fuzz uses; CI only |

**Supply-chain rules applied:**
- every crate version is pinned in `fuzz/Cargo.lock`;
- `libfuzzer-sys` is pinned exactly (`=0.4.13`);
- **every locked version is at least two weeks old.** Three newer releases were deliberately avoided: `arbitrary` 1.5.0, `cc` 1.8.0 and `libc` 0.2.191, all published in the last 11 days;
- all crates come from crates.io, maintained by the rust-fuzz project (the standard Rust fuzzing tooling) and the Rust project.

## Native code (D-005, D-015)

`libfuzzer-sys` compiles **LLVM's libFuzzer, which is C++**, into the fuzz binaries.
- **This is the one non-Rust component.** It is acceptable here because it is test tooling, never Septuvo code and never shipped.
- **D-015's "no hidden C/C++ cryptographic engine" is not affected:** libFuzzer contains no cryptography.
- It is recorded here and in `fuzz/NOTICE`.

## Risks and handling

| Risk | Handling |
| --- | --- |
| A compromised or malicious crate release | Exact versions in the lockfile, at least two weeks old; dependencies run only on disposable CI runners and developer machines, with no secrets and `contents: read` permissions |
| Nightly toolchain instability | One pinned nightly date; a breakage only affects the fuzz jobs, never the product build |
| Fuzz code drifting from the product's real checks | The cargo-fuzz targets call **the same property-check functions** as the standard-library fuzz tests (`crates/*/tests/fuzz.rs`) |
| Reliance on it for security | It finds bugs; it proves nothing. The G1 and G3 independent reviews are unchanged |

## Alternatives considered

- **AFL++ (`afl.rs`).** Similar strength, but a heavier toolchain. Not needed alongside libFuzzer.
- **`honggfuzz`.** Fine, but less common in Rust projects.
- **Staying standard-library only.** Kept as the per-pull-request check, which runs everywhere. Coverage guidance finds deeper bugs faster, which is why it is added on top.
