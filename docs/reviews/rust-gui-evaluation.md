# Rust-native GUI evaluation for the Septuvo VPN desktop app (N4)

**Prepared 2026-10-17** after the owner rejected GTK 4 + libadwaita (a C application framework with Rust bindings) for the Septuvo VPN desktop app.

> **Context update, 2026-10-18.** The owner has since clarified D-005: it requires **all Septuvo-authored VPN and Server code to be Rust**, and does **not** restrict the language of external components Septuvo calls or interacts with (each is still reviewed). The rule text this evaluation was written against ("no bindings around a C/C++ framework, no bundled non-Rust engine") is therefore no longer D-005. Later the same day the owner also said GTK is acceptable if it is the best fit. The evaluation and its result stand on their merits: egui/eframe is the N4 toolkit (N4-D1), because the approved desktop designs are custom-drawn and shared across Windows, Linux and macOS ([desktop-ui-reconciliation.md](desktop-ui-reconciliation.md) section 4). The native-code findings below are what N4-01's review and gate list.

- **The criteria used here:** a toolkit written in Rust, using the operating system's own APIs, services, drivers and graphical interfaces.
- **Excluded here:** Rust bindings around a C/C++ application framework, or a bundled non-Rust application engine.

This record is the evidence behind N4-D1. The owner's merge of the N4 work order adopts its recommendation.

## Method

For each candidate, a minimal project was created with exact versions and realistic Linux features. Its dependency tree was then **resolved for `x86_64-unknown-linux-gnu`** (`cargo tree --target … -e normal,build`), so Windows and macOS crates are excluded. Every crate in it was scanned for:
- `links =` (native library linking);
- build scripts using `cc`, `cmake`, `pkg-config`, `bindgen` or `system-deps`;
- C, C++ or Objective-C source files;
- `#[link(name = …)]`;
- library names loaded at runtime (`dlopen`).

**Every hit was then checked by hand,** including which features are actually enabled. Finally, the leading candidate was **built** and its binary inspected:
- `readelf -d` for the libraries linked at load time;
- the build directory for any compiled C objects;
- `strings` for library names loaded at runtime.

## Results

| Candidate (exact version) | Linux crates | Compiled C/C++ in the app | Native libraries | Accessibility (screen readers) | Verdict |
| --- | ---: | --- | --- | --- | --- |
| **egui / eframe 0.36.2** (glow renderer, AccessKit, Wayland and X11) | 206 | **None.** The binary links only `libc`, `libm`, `libgcc_s` and the loader, exactly like `septuvod` | Loaded at runtime from the OS: display server (`libwayland-client`; Xlib/xcb), keyboard layouts (`libxkbcommon`), OpenGL driver (`libEGL`/`libGL`) | **Yes:** AccessKit (pure Rust) → AT-SPI over D-Bus (`zbus`, pure Rust), on by default | **Recommended** |
| iced 0.14.0 (tiny-skia CPU renderer) | 149 | None | Display server and keyboard only (no GPU needed) | **No.** iced 0.14 has no accessibility support upstream (no AccessKit feature) | Not recommended: fails N4's screen-reader requirement. Re-evaluate if upstream adds AccessKit |
| iced 0.14.0 (wgpu) | 237 | None | As above, plus the Vulkan/GL drivers | No | As above |
| Slint 1.18.1 (software renderer) | 332 | None | Display server, keyboard, **fontconfig** | Yes (AccessKit); also a built-in tray | **Rejected.** The UI is written in Slint's own markup language (`.slint`), so Septuvo's UI code would not be Rust. Its licence choices (GPL-3.0, royalty-free with attribution, commercial) do not fit D-033 |
| Xilem / Masonry 0.4.0 | 257 | None | Display server, keyboard, Vulkan/GL (vello needs a GPU), **fontconfig** | Yes (AccessKit) | Not now: alpha, frequent breaking changes, few widgets. Watch it |
| Floem 0.2.0 | — | — | — | — | Not now: no release since 2024-11 |
| GTK 4 / libadwaita (gtk-rs), Relm4 | — | GTK, GLib, Pango and Cairo are C | — | — | Rejected by the owner on 2026-10-17; **permitted since 2026-10-18**, not chosen (theme-driven, cannot match the custom designs consistently across platforms) |
| Freya (Skia), Tauri / Dioxus desktop (web view) | — | Bundle a C++ engine (Skia, WebKit) | — | — | **Not chosen** (bundled non-Rust engine) |

**Desktop services**, for the tray, notifications and file saving: `ksni` 0.3.6, `notify-rust` 4.18.1 (with its `zbus` backend) and `ashpd` 0.13.13 (the xdg desktop portal).
- **Compiled C:** none, across 81 Linux crates.
- **Native code:** nothing beyond `libc`.
- **How they talk to the desktop:** they use the D-Bus protocol in pure Rust, reaching the desktop's own services (tray host, notification daemon, file-chooser portal), which run as separate processes.

## What the hand checks showed

| Hit | Finding |
| --- | --- |
| `wayland-backend` build script uses `cc` and has `log_shim.c` | The C file is compiled **only with the `log` feature**, which no candidate enables. Nothing is compiled; the binary build confirms this (no `.o` or `.a` produced) |
| `ttf-parser`: 12 C++ files | Developer test tools (`testing-tools/font-view`), never compiled |
| `wayland-protocols-plasma`: 1 C file | An upstream protocol test (`autotests/build.c`), never compiled |
| `wayland-sys` (`client_system`, `dlopen`) | Loads the system's `libwayland-client` at runtime: the display server's client interface |
| `x11-dl`, `tiny-xlib`, `x11rb` (`dl-libxcb`) | Load the system's Xlib/xcb at runtime: the X11 display server's client interface |
| `xkbcommon-dl` | Loads the system's `libxkbcommon` at runtime: the keyboard-layout service every Linux desktop provides (Wayland sends keymaps in its format) |
| `glutin`, `khronos-egl` (eframe glow) | Load the OpenGL driver (`libEGL`/`libGL`) at runtime; Mesa provides a software fallback on machines without a GPU |
| `libc` | The system C library. **Every Rust program on Linux links it**, `septuvod` included: it is how Rust's standard library reaches the kernel |
| `link:` entries for Windows and macOS frameworks | Behind `cfg` for other operating systems; not compiled on Linux |

## Classification

- **Operating-system interfaces (allowed):**
  - the kernel and the system C library;
  - the display server's client libraries (Wayland, X11);
  - the keyboard-layout library;
  - graphics drivers (OpenGL/Vulkan, including Mesa);
  - desktop services reached over D-Bus (tray host, notifications, portals, the AT-SPI accessibility bus).

  These are loaded from the user's system, are not bundled, are not Septuvo code, and are what every native desktop application on Linux uses.
- **Optional non-Rust application dependencies (excluded by this evaluation's criteria):** **none** in the recommended stack. GTK/libadwaita, Qt, Skia, WebKit, Chromium and SDL would all fall here, and all are excluded.

## Why egui / eframe

1. **Rust all the way to the OS:** no C or C++ is compiled or bundled. The only native code is the operating system's own (above).
2. **Accessibility today:** AccessKit is built in and on by default, giving screen readers, keyboard focus and accessible names over AT-SPI.
3. **Maintainability:**
   - very active (monthly releases);
   - widely used (Rerun, many tools);
   - MIT OR Apache-2.0;
   - a well-defined, small API.
4. **Testability:** `egui_kittest`, a Rust test harness from the same project, drives the UI **through the accessibility tree** with no display. That is ideal for CI, and it doubles as an accessibility check. It is evaluated in N4-01 as a test-only dependency.

**Known trade-offs**, handled in the work order:
- **Appearance:** egui's default look is utilitarian. A polished consumer look takes deliberate work: a Septuvo theme from the brand tokens, the Inter font (font data under its own licence), spacing, and icons. N4-01 must show a styled prototype for the owner's approval before more is built.
- **Layout model:** immediate mode, so responsive layouts are written explicitly (width breakpoints). This is planned in N4-02.
- **Graphics:** it needs an OpenGL driver; Mesa's software rendering covers machines and VMs without a GPU.
- **Text:** it does not do full complex-script shaping (for example Arabic). That is fine for the English-first UI, and is to be re-evaluated before localization.
- **Tray on GNOME:** StatusNotifierItem needs the AppIndicator extension there; without it the app still works as a window.

## Versions

The exact pins are chosen in N4-01's dependency review, with **every locked version at least two weeks old** (the D-035 rule). This audit used eframe/egui 0.36.2 (published 2026-09-08), ksni 0.3.6, notify-rust 4.18.1 and ashpd 0.13.13. Some of their transitive dependencies resolved to newer releases, which N4-01 pins back.
