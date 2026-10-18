# Dependency review: Septuvo VPN desktop app (N4-01)

**Prepared 2026-10-18 for N4-01**, under the N4 work order (N4-D1 to N4-D5) and D-015. Written **before** the crates were added to the repository. **Scope:** the `desktop/` workspace only. The main workspace (`crates/`) still has **no third-party crates**, and CI keeps checking that.

**The rule this follows (D-005, D-015):**
- All Septuvo-authored code is Rust.
- External components may be in any language, but each is reviewed and listed before adoption.
- This record is that review for the desktop app. `desktop/native-allowlist.txt` is its machine-checked part.

## 1. Direct dependencies

| Crate | Version (exact pin) | Published | Licence | Why | Shipped? |
| --- | --- | --- | --- | --- | --- |
| `eframe` (with `egui`) | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 | The UI toolkit (N4-D1). Features: `accesskit` (screen readers), `glow` (OpenGL renderer), `wayland`, `x11`. **Default features off**, so no default fonts, no persistence, no web or wgpu code | Yes |
| `egui_kittest` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 | UI tests through the accessibility tree, without a display. Default features off (no snapshot or GPU renderer) | **No: tests only** |
| `septuvo-cli`, `septuvo-ipc` | path | — | Apache-2.0 | Septuvo's own status model and IPC client, reused rather than copied | Yes |

**Not added yet:** the tray (`ksni`), notification (`notify-rust`) and file-save (`ashpd`) crates from N4-D2/N4-D3. They come with N4-03 and N4-06. Each will be pinned and added to this record **in that task's pull request**, under the same two-week rule and the same gate.

## 2. The whole tree

| Measure | Result |
| --- | --- |
| Crates built for `x86_64-unknown-linux-gnu` (normal, build and dev dependencies, with the enabled features) | **210 third-party crates**, plus 4 Septuvo crates |
| Two-week rule (D-035) | **Every locked version was published on or before 2026-10-04**. The newest is from 2026-10-03; 26 locked crates were pinned back from newer releases |
| Pinning | `desktop/Cargo.lock`; CI uses `--locked` |
| Licences | All permissive (below). Where a crate offers a choice that includes a copyleft licence (`self_cell`: Apache-2.0 OR GPL-2.0-only), **Septuvo uses the permissive option** |
| Compiled C/C++ | **None.** No build script compiles anything (`cc` is present but inert; see section 3). The CI gate also checks the build directory after a real build |
| The built binary needs (ELF `DT_NEEDED`) | `libc.so.6`, `libm.so.6`, `libgcc_s.so.1`, `ld-linux-x86-64.so.2`: the system C library, as for every Rust program including `septuvod` |
| Loaded at run time | The system's display server libraries (Wayland or X11), keyboard layouts (`libxkbcommon`) and OpenGL driver (`libEGL`/`libGL`, Mesa or the GPU vendor). All operating-system libraries, not bundled |
| Desktop services | Reached over D-Bus by `zbus` (pure Rust): the AT-SPI accessibility bus today; tray, notifications and portals later |

Licences across the 210 crates:

| Licence expression | Crates |
| --- | ---: |
| MIT OR Apache-2.0 (incl. `Apache-2.0 OR MIT`, `MIT/Apache-2.0`) | 130 |
| MIT | 48 |
| Apache-2.0 | 9 |
| MIT OR Apache-2.0 OR Zlib (any order) | 8 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 4 |
| Unlicense OR MIT | 2 |
| BSD-3-Clause OR Apache-2.0 | 2 |
| 0BSD OR MIT OR Apache-2.0; BSD-2-Clause OR Apache-2.0 OR MIT; Apache-2.0 AND MIT; Zlib; ISC; (MIT OR Apache-2.0) AND Unicode-3.0 | 1 each |
| Apache-2.0 OR GPL-2.0-only (`self_cell`; Apache-2.0 used) | 1 |

Every crate, with its version, publish date and licence, is listed in the appendix.

## 3. Non-Rust native code: every finding and its reason

The gate (`crates/septuvo-native-gate`) inspected all 214 crates. These are all its findings; each one is an entry in `desktop/native-allowlist.txt`.

| Finding | Crates | Why it is acceptable |
| --- | --- | --- |
| Native build tool present | `cc` (build dependency of `wayland-backend`) | Used only with `wayland-backend`'s `log` feature, which is **off**: nothing is compiled. The gate fails if the feature is ever turned on |
| Native build tool present | `pkg-config` (`wayland-sys`, `x11-dl`) | `wayland-sys`: unused with its `dlopen` feature (on). `x11-dl`: only asks where the system's X11 libraries are installed, to load them at run time; compiles and links nothing |
| C source files in a crate | `cc`, `wayland-backend`, `wayland-protocols-plasma` | Never compiled: no compiler runs in this build (reported as "inert") |
| Libraries named for linking | `libc`, `x11-dl` | The system C library on Linux (`c`, `m`, `rt`, `pthread`, `dl`, `util`) |
| Libraries named only for other systems | `libc`, `getrandom`, `find-msvc-tools`, `winit`, `x11rb` | Inside `cfg` blocks for Windows, macOS, BSD, Fuchsia or illumos (or a disabled feature); never linked on Linux |
| Bundled binaries | `libloading` (`tests/nagisa32.dll`, `tests/nagisa64.dll`) | Windows test fixtures in the crate's tests directory; never built, shipped or loaded |
| Library names in strings | `wayland-sys`, `x11-dl`, `x11rb`, `xkbcommon-dl`, `glutin` | The OS display, keyboard and OpenGL libraries, loaded at run time from the system. `pkg-config`'s `libfoo.so` is a unit-test string |

**Excluded by the N4 toolkit decision (N4-D1):** GTK, libadwaita, Qt, Skia, web views and SDL crates. The gate fails on any of them, whatever the allowlist says.

## 4. Bundled assets (data, not code)

| File | Source | Licence |
| --- | --- | --- |
| `desktop/assets/fonts/Inter-Regular.ttf`, `Inter-SemiBold.ttf` | Inter 4.1 release archive from github.com/rsms/inter (archive SHA-256 `9883fdd4…b11e`), files unmodified | SIL Open Font License 1.1 (`desktop/assets/fonts/OFL.txt`). The brand guide names Inter as the UI typeface; N4-D6 approved bundling it as font data |
| `desktop/assets/icons/septuvo-vpn-128.rgba` | `brand/03 - App Icons/VPN/VPN_Dark_128.png`, converted to raw RGBA so the app needs no image decoder | Septuvo brand asset (`brand/NOTICE.md`) |

## 5. Risks and how they are handled

| Risk | Handling |
| --- | --- |
| A future update adds native code | Exact pins; the CI gate over the whole tree and the built binary; any allowlist change is a reviewed pull request needing the owner's approval |
| A large tree (210 crates) | Mostly the window system, text and accessibility. All permissive, all at least two weeks old. Updates are deliberate, never automatic |
| `unsafe` code inside dependencies | Expected for a window toolkit (OS calls). Septuvo's own app code forbids `unsafe` (`unsafe_code = "forbid"`) |
| OpenGL driver problems | Mesa's software renderer works without a GPU; CI runs that way |

## 6. Updating

1. Change a pin deliberately (`cargo update -p NAME --precise VERSION`), keeping the two-week rule.
2. Run the gate. Add or change allowlist entries **only** with a reason.
3. Update this record in the same pull request.

## Appendix: every third-party crate built for Linux

| Crate | Version | Published | Licence |
| --- | --- | --- | --- |
| `accesskit` | 0.24.1 | 2026-06-12 | MIT OR Apache-2.0 |
| `accesskit_atspi_common` | 0.18.1 | 2026-05-11 | MIT OR Apache-2.0 |
| `accesskit_consumer` | 0.35.0 | 2026-03-04 | MIT OR Apache-2.0 |
| `accesskit_consumer` | 0.36.0 | 2026-05-11 | MIT OR Apache-2.0 |
| `accesskit_unix` | 0.21.1 | 2026-05-11 | MIT OR Apache-2.0 |
| `accesskit_winit` | 0.32.2 | 2026-03-04 | Apache-2.0 |
| `adler2` | 2.0.1 | 2025-06-09 | 0BSD OR MIT OR Apache-2.0 |
| `ahash` | 0.8.12 | 2025-05-08 | MIT OR Apache-2.0 |
| `arboard` | 3.6.1 | 2025-08-23 | MIT OR Apache-2.0 |
| `arrayvec` | 0.7.8 | 2026-07-02 | MIT OR Apache-2.0 |
| `as-raw-xcb-connection` | 1.0.1 | 2023-12-17 | MIT OR Apache-2.0 |
| `async-broadcast` | 0.7.2 | 2024-12-23 | MIT OR Apache-2.0 |
| `async-channel` | 2.5.0 | 2025-07-06 | Apache-2.0 OR MIT |
| `async-executor` | 1.14.0 | 2026-02-15 | Apache-2.0 OR MIT |
| `async-io` | 2.6.0 | 2025-09-14 | Apache-2.0 OR MIT |
| `async-lock` | 3.4.2 | 2025-12-21 | Apache-2.0 OR MIT |
| `async-process` | 2.5.0 | 2025-09-14 | Apache-2.0 OR MIT |
| `async-recursion` | 1.2.0 | 2026-10-03 | MIT OR Apache-2.0 |
| `async-signal` | 0.2.14 | 2026-04-07 | Apache-2.0 OR MIT |
| `async-task` | 4.7.1 | 2024-04-27 | Apache-2.0 OR MIT |
| `async-trait` | 0.1.92 | 2026-08-08 | MIT OR Apache-2.0 |
| `atomic-waker` | 1.1.2 | 2023-09-25 | Apache-2.0 OR MIT |
| `atspi` | 0.29.0 | 2025-09-18 | Apache-2.0 OR MIT |
| `atspi-common` | 0.13.0 | 2025-09-18 | Apache-2.0 OR MIT |
| `atspi-proxies` | 0.13.0 | 2025-09-18 | Apache-2.0 OR MIT |
| `autocfg` | 1.5.1 | 2026-05-22 | Apache-2.0 OR MIT |
| `bitflags` | 2.13.2 | 2026-09-10 | MIT OR Apache-2.0 |
| `blocking` | 1.7.0 | 2026-08-23 | Apache-2.0 OR MIT |
| `bytemuck` | 1.25.2 | 2026-07-19 | Zlib OR Apache-2.0 OR MIT |
| `bytemuck_derive` | 1.12.1 | 2026-09-11 | Zlib OR Apache-2.0 OR MIT |
| `byteorder-lite` | 0.1.0 | 2024-04-07 | Unlicense OR MIT |
| `calloop` | 0.13.0 | 2024-03-01 | MIT |
| `calloop` | 0.14.5 | 2026-10-01 | MIT |
| `calloop-wayland-source` | 0.3.0 | 2024-03-01 | MIT |
| `calloop-wayland-source` | 0.4.1 | 2025-09-06 | MIT |
| `cc` | 1.6.0 | 2026-10-03 | MIT OR Apache-2.0 |
| `cfg-if` | 1.0.5 | 2026-09-16 | MIT OR Apache-2.0 |
| `cfg_aliases` | 0.2.2 | 2026-07-16 | MIT |
| `color` | 0.3.3 | 2026-05-05 | Apache-2.0 OR MIT |
| `concurrent-queue` | 2.5.0 | 2024-04-26 | Apache-2.0 OR MIT |
| `crc32fast` | 1.5.2 | 2026-09-12 | MIT OR Apache-2.0 |
| `crossbeam-utils` | 0.8.23 | 2026-09-05 | MIT OR Apache-2.0 |
| `cursor-icon` | 1.2.0 | 2025-05-25 | MIT OR Apache-2.0 OR Zlib |
| `dlib` | 0.5.3 | 2026-02-26 | MIT |
| `document-features` | 0.2.12 | 2025-10-24 | MIT OR Apache-2.0 |
| `downcast-rs` | 1.2.1 | 2024-04-07 | MIT/Apache-2.0 |
| `dpi` | 0.1.2 | 2025-05-02 | Apache-2.0 AND MIT |
| `ecolor` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 |
| `eframe` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 |
| `egui` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 |
| `egui-winit` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 |
| `egui_glow` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 |
| `egui_kittest` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 |
| `either` | 1.18.0 | 2026-08-20 | MIT OR Apache-2.0 |
| `emath` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 |
| `endi` | 1.1.1 | 2025-11-26 | MIT |
| `enumflags2` | 0.7.12 | 2025-06-09 | MIT OR Apache-2.0 |
| `enumflags2_derive` | 0.7.12 | 2025-06-09 | MIT OR Apache-2.0 |
| `epaint` | 0.36.2 | 2026-09-08 | MIT OR Apache-2.0 |
| `equivalent` | 1.0.2 | 2025-02-14 | Apache-2.0 OR MIT |
| `errno` | 0.3.14 | 2025-09-09 | MIT OR Apache-2.0 |
| `euclid` | 0.22.14 | 2026-03-18 | MIT OR Apache-2.0 |
| `event-listener` | 5.4.2 | 2026-07-27 | Apache-2.0 OR MIT |
| `event-listener-strategy` | 0.5.4 | 2025-03-27 | Apache-2.0 OR MIT |
| `fastrand` | 2.5.0 | 2026-07-19 | Apache-2.0 OR MIT |
| `fdeflate` | 0.3.7 | 2024-12-06 | MIT OR Apache-2.0 |
| `fearless_simd` | 0.4.1 | 2026-05-16 | Apache-2.0 OR MIT |
| `find-msvc-tools` | 0.1.14 | 2026-09-25 | MIT OR Apache-2.0 |
| `flate2` | 1.1.10 | 2026-08-28 | MIT OR Apache-2.0 |
| `foldhash` | 0.2.0 | 2025-08-23 | Zlib |
| `font-types` | 0.12.6 | 2026-10-03 | MIT OR Apache-2.0 |
| `futures-core` | 0.3.34 | 2026-08-11 | MIT OR Apache-2.0 |
| `futures-io` | 0.3.34 | 2026-08-11 | MIT OR Apache-2.0 |
| `futures-lite` | 2.6.1 | 2025-08-04 | Apache-2.0 OR MIT |
| `futures-macro` | 0.3.34 | 2026-08-11 | MIT OR Apache-2.0 |
| `futures-task` | 0.3.34 | 2026-08-11 | MIT OR Apache-2.0 |
| `futures-util` | 0.3.34 | 2026-08-11 | MIT OR Apache-2.0 |
| `gethostname` | 1.1.0 | 2025-10-14 | Apache-2.0 |
| `getrandom` | 0.3.4 | 2025-10-14 | MIT OR Apache-2.0 |
| `gl_generator` | 0.14.0 | 2019-10-03 | Apache-2.0 |
| `glow` | 0.17.0 | 2026-03-07 | MIT OR Apache-2.0 OR Zlib |
| `glutin` | 0.32.3 | 2025-04-30 | Apache-2.0 |
| `glutin-winit` | 0.5.0 | 2024-06-08 | MIT |
| `glutin_egl_sys` | 0.7.1 | 2025-01-12 | Apache-2.0 |
| `glutin_glx_sys` | 0.6.1 | 2025-01-12 | Apache-2.0 |
| `guillotiere` | 0.7.0 | 2026-03-18 | MIT/Apache-2.0 |
| `harfrust` | 0.12.0 | 2026-07-03 | MIT |
| `hashbrown` | 0.16.1 | 2025-11-20 | MIT OR Apache-2.0 |
| `hashbrown` | 0.17.1 | 2026-05-09 | MIT OR Apache-2.0 |
| `hex` | 0.4.3 | 2021-03-03 | MIT OR Apache-2.0 |
| `image` | 0.25.10 | 2026-03-10 | MIT OR Apache-2.0 |
| `indexmap` | 2.14.2 | 2026-09-05 | Apache-2.0 OR MIT |
| `itertools` | 0.15.0 | 2026-06-16 | MIT OR Apache-2.0 |
| `khronos_api` | 3.1.0 | 2019-04-01 | Apache-2.0 |
| `kittest` | 0.4.0 | 2026-03-24 | MIT OR Apache-2.0 |
| `kurbo` | 0.13.1 | 2026-05-13 | Apache-2.0 OR MIT |
| `libc` | 0.2.190 | 2026-10-02 | MIT OR Apache-2.0 |
| `libloading` | 0.8.9 | 2025-09-21 | ISC |
| `linebender_resource_handle` | 0.1.1 | 2025-09-16 | Apache-2.0 OR MIT |
| `linux-raw-sys` | 0.12.1 | 2025-12-23 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| `linux-raw-sys` | 0.4.15 | 2025-01-08 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| `litrs` | 1.0.0 | 2025-10-20 | MIT OR Apache-2.0 |
| `lock_api` | 0.4.14 | 2025-10-03 | MIT OR Apache-2.0 |
| `log` | 0.4.34 | 2026-08-22 | MIT OR Apache-2.0 |
| `memchr` | 2.8.3 | 2026-07-08 | Unlicense OR MIT |
| `memmap2` | 0.9.11 | 2026-06-22 | MIT OR Apache-2.0 |
| `miniz_oxide` | 0.8.9 | 2025-06-09 | MIT OR Zlib OR Apache-2.0 |
| `miniz_oxide` | 0.9.1 | 2026-03-13 | MIT OR Zlib OR Apache-2.0 |
| `moxcms` | 0.8.1 | 2026-03-07 | BSD-3-Clause OR Apache-2.0 |
| `nohash-hasher` | 0.2.0 | 2020-02-17 | Apache-2.0 OR MIT |
| `num-traits` | 0.2.19 | 2024-05-03 | MIT OR Apache-2.0 |
| `once_cell` | 1.21.4 | 2026-03-12 | MIT OR Apache-2.0 |
| `ordered-stream` | 0.2.0 | 2023-02-11 | MIT OR Apache-2.0 |
| `parking` | 2.2.1 | 2024-09-07 | Apache-2.0 OR MIT |
| `parking_lot` | 0.12.5 | 2025-10-03 | MIT OR Apache-2.0 |
| `parking_lot_core` | 0.9.12 | 2025-10-03 | MIT OR Apache-2.0 |
| `peniko` | 0.6.1 | 2026-05-15 | Apache-2.0 OR MIT |
| `percent-encoding` | 2.3.2 | 2025-08-21 | MIT OR Apache-2.0 |
| `phf` | 0.13.1 | 2025-08-23 | MIT |
| `phf_generator` | 0.13.1 | 2025-08-23 | MIT |
| `phf_macros` | 0.13.1 | 2025-08-23 | MIT |
| `phf_shared` | 0.13.1 | 2025-08-23 | MIT |
| `pin-project-lite` | 0.2.17 | 2026-02-27 | Apache-2.0 OR MIT |
| `piper` | 0.2.5 | 2026-02-28 | MIT OR Apache-2.0 |
| `pkg-config` | 0.3.34 | 2026-08-14 | MIT OR Apache-2.0 |
| `png` | 0.18.1 | 2026-02-14 | MIT OR Apache-2.0 |
| `polling` | 3.11.0 | 2025-09-14 | Apache-2.0 OR MIT |
| `polycool` | 0.4.0 | 2025-10-23 | MIT OR Apache-2.0 |
| `proc-macro-crate` | 3.5.0 | 2026-03-04 | MIT OR Apache-2.0 |
| `proc-macro2` | 1.0.107 | 2026-07-19 | MIT OR Apache-2.0 |
| `profiling` | 1.0.18 | 2026-05-04 | MIT OR Apache-2.0 |
| `pxfm` | 0.1.30 | 2026-07-06 | BSD-3-Clause OR Apache-2.0 |
| `quick-xml` | 0.41.0 | 2026-06-29 | MIT |
| `quote` | 1.0.47 | 2026-07-19 | MIT OR Apache-2.0 |
| `raw-window-handle` | 0.6.2 | 2024-05-17 | MIT OR Apache-2.0 OR Zlib |
| `read-fonts` | 0.41.0 | 2026-07-03 | MIT OR Apache-2.0 |
| `rustix` | 0.38.44 | 2025-01-21 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| `rustix` | 1.1.5 | 2026-09-16 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| `scoped-tls` | 1.0.1 | 2022-10-31 | MIT/Apache-2.0 |
| `scopeguard` | 1.2.0 | 2023-07-17 | MIT OR Apache-2.0 |
| `self_cell` | 1.3.0 | 2026-07-16 | Apache-2.0 OR GPL-2.0-only |
| `serde` | 1.0.229 | 2026-07-18 | MIT OR Apache-2.0 |
| `serde_core` | 1.0.229 | 2026-07-18 | MIT OR Apache-2.0 |
| `serde_derive` | 1.0.229 | 2026-07-18 | MIT OR Apache-2.0 |
| `serde_repr` | 0.1.21 | 2026-07-18 | MIT OR Apache-2.0 |
| `serde_spanned` | 1.1.1 | 2026-03-31 | MIT OR Apache-2.0 |
| `shlex` | 2.0.1 | 2026-05-17 | MIT OR Apache-2.0 |
| `signal-hook-registry` | 1.4.8 | 2025-12-25 | MIT OR Apache-2.0 |
| `simd-adler32` | 0.3.10 | 2026-07-14 | MIT |
| `siphasher` | 1.0.4 | 2026-09-25 | MIT OR Apache-2.0 |
| `skrifa` | 0.44.0 | 2026-07-04 | MIT OR Apache-2.0 |
| `slab` | 0.4.12 | 2026-01-31 | MIT |
| `smallvec` | 1.16.2 | 2026-09-25 | MIT OR Apache-2.0 |
| `smithay-client-toolkit` | 0.19.2 | 2024-07-15 | MIT |
| `smithay-client-toolkit` | 0.20.0 | 2025-08-08 | MIT |
| `smithay-clipboard` | 0.7.3 | 2025-11-02 | MIT |
| `smol_str` | 0.2.2 | 2024-05-14 | MIT OR Apache-2.0 |
| `static_assertions` | 1.1.0 | 2019-11-03 | MIT OR Apache-2.0 |
| `syn` | 2.0.119 | 2026-07-15 | MIT OR Apache-2.0 |
| `syn` | 3.0.6 | 2026-09-16 | MIT OR Apache-2.0 |
| `thiserror` | 1.0.69 | 2024-11-10 | MIT OR Apache-2.0 |
| `thiserror` | 2.0.21 | 2026-09-23 | MIT OR Apache-2.0 |
| `thiserror-impl` | 1.0.69 | 2024-11-10 | MIT OR Apache-2.0 |
| `thiserror-impl` | 2.0.21 | 2026-09-23 | MIT OR Apache-2.0 |
| `toml` | 1.1.6+spec-1.1.0 | 2026-09-10 | MIT OR Apache-2.0 |
| `toml_datetime` | 1.1.1+spec-1.1.0 | 2026-03-31 | MIT OR Apache-2.0 |
| `toml_edit` | 0.25.15+spec-1.1.0 | 2026-09-11 | MIT OR Apache-2.0 |
| `toml_parser` | 1.1.3+spec-1.1.0 | 2026-07-27 | MIT OR Apache-2.0 |
| `tracing` | 0.1.44 | 2025-12-18 | MIT |
| `tracing-attributes` | 0.1.31 | 2025-11-26 | MIT |
| `tracing-core` | 0.1.36 | 2025-12-18 | MIT |
| `unicode-general-category` | 1.1.0 | 2025-09-16 | Apache-2.0 |
| `unicode-ident` | 1.0.26 | 2026-09-17 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| `unicode-segmentation` | 1.13.3 | 2026-06-01 | MIT OR Apache-2.0 |
| `uuid` | 1.27.0 | 2026-10-02 | Apache-2.0 OR MIT |
| `vello_common` | 0.1.0 | 2026-07-29 | Apache-2.0 OR MIT |
| `vello_cpu` | 0.1.0 | 2026-07-29 | Apache-2.0 OR MIT |
| `version_check` | 0.9.5 | 2024-07-25 | MIT/Apache-2.0 |
| `wayland-backend` | 0.3.17 | 2026-08-14 | MIT |
| `wayland-client` | 0.31.15 | 2026-07-22 | MIT |
| `wayland-csd-frame` | 0.3.0 | 2023-09-16 | MIT |
| `wayland-cursor` | 0.31.14 | 2026-03-31 | MIT |
| `wayland-protocols` | 0.32.13 | 2026-06-19 | MIT |
| `wayland-protocols-experimental` | 20250721.0.1 | 2025-07-29 | MIT |
| `wayland-protocols-misc` | 0.3.12 | 2026-03-31 | MIT |
| `wayland-protocols-plasma` | 0.3.12 | 2026-03-31 | MIT |
| `wayland-protocols-wlr` | 0.3.12 | 2026-03-31 | MIT |
| `wayland-scanner` | 0.31.11 | 2026-07-22 | MIT |
| `wayland-sys` | 0.31.11 | 2026-03-31 | MIT |
| `web-time` | 1.1.0 | 2024-03-01 | MIT OR Apache-2.0 |
| `winit` | 0.30.13 | 2026-03-02 | Apache-2.0 |
| `winnow` | 1.0.4 | 2026-07-13 | MIT |
| `x11-dl` | 2.21.0 | 2023-01-18 | MIT |
| `x11rb` | 0.13.2 | 2025-08-29 | MIT OR Apache-2.0 |
| `x11rb-protocol` | 0.13.2 | 2025-08-29 | MIT OR Apache-2.0 |
| `xcursor` | 0.3.11 | 2026-07-31 | MIT |
| `xkbcommon-dl` | 0.4.2 | 2024-02-11 | MIT |
| `xkeysym` | 0.2.1 | 2024-06-07 | MIT OR Apache-2.0 OR Zlib |
| `xml-rs` | 0.8.29 | 2026-08-06 | MIT |
| `zbus` | 5.19.0 | 2026-08-09 | MIT |
| `zbus-lockstep` | 0.5.2 | 2025-11-29 | MIT |
| `zbus-lockstep-macros` | 0.5.2 | 2025-11-29 | MIT |
| `zbus_macros` | 5.19.0 | 2026-08-09 | MIT |
| `zbus_names` | 4.3.4 | 2026-07-17 | MIT |
| `zbus_xml` | 5.2.1 | 2026-07-17 | MIT |
| `zcheapstr` | 1.1.0 | 2026-08-10 | MIT |
| `zerocopy` | 0.8.59 | 2026-09-25 | BSD-2-Clause OR Apache-2.0 OR MIT |
| `zvariant` | 5.15.0 | 2026-08-18 | MIT |
| `zvariant_derive` | 5.15.0 | 2026-08-18 | MIT |
| `zvariant_utils` | 4.2.0 | 2026-08-18 | MIT |
