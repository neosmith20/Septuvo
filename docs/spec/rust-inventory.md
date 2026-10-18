# Rust-only inventory and open platform questions (D-005)

**The rule (D-005, owner decided 2026-10-17):**
- **Septuvo VPN and Septuvo Server: 100% Rust, without exceptions.** That covers every client, user interface, service, server-administration tool, installer and bundled application-level dependency.
- **Allowed:** using the operating system's own APIs, services, drivers and graphical interfaces. **Not allowed:** Rust bindings around a C/C++ application framework, or any bundled non-Rust application engine.
- **Septuvo Protocol (SVP): Rust preferred, including its cryptography.** A narrowly scoped non-Rust cryptographic component may only be *presented* to the owner after a qualified security reviewer finds it necessary or significantly safer. It needs the owner's explicit approval before implementation.

This file is how the rule is checked. It records:
- everything in the repository that is not Rust today, and its status;
- the operating-system interfaces Septuvo uses;
- the places where a future platform may make Rust-only hard. **Those are open questions, not exceptions.**

## 1. Non-Rust files in the repository today

| Path | What | Shipped with Septuvo VPN or Server? | Status under D-005 |
| --- | --- | --- | --- |
| `packaging/linux/install.sh`, `uninstall.sh` | POSIX shell installer (N3-01) | **Yes** | **Must be replaced by a Rust installer.** Scheduled in N4-07; the shell scripts are removed when it lands |
| `packaging/linux/*.service`, `*.sysusers`, `*.tmpfiles`, `config/*.toml` | systemd unit, sysusers and tmpfiles files; configuration templates | Yes | **Data for the OS service manager and Septuvo's parsers, not code.** Allowed (OS interface) |
| `scripts/*.py`, `scripts/tests/*.py` | Repository checks (ledger, licensing, links) | No: development and CI tooling | Not part of Septuvo VPN or Server. **Owner to confirm** whether to keep or migrate to Rust |
| `tests/lab/**/*.sh`, `tests/lab/namespaces/probe.py` | Lab test harnesses (namespaces, installer) | No: test tooling | As above |
| `fuzz/ci-run.sh`; libFuzzer (C++, inside `libfuzzer-sys`, D-035) | Fuzzing runner and engine | No: development only, never built into or shipped with Septuvo | As above. D-035 stays as recorded unless the owner decides otherwise |
| `.github/workflows/*.yml` | CI configuration | No | CI configuration, not code |

**Septuvo-owned product code** (`crates/`, and the coming `desktop/`) **is 100% Rust.** The main workspace has no third-party crates, and the desktop app's dependency tree is gated by N4-01's Rust-only check.

## 2. Operating-system interfaces Septuvo uses (allowed)

| Interface | Used by | Notes |
| --- | --- | --- |
| Linux kernel (system calls, TUN, netlink, namespaces) | Everything | Through Rust's standard library |
| System C library (`libc`, `libm`, `libgcc_s`, loader) | **Every Rust binary on Linux**, `septuvod` included | How Rust's standard library reaches the kernel; not bundled |
| `nft`, `ip`, `systemd`, `systemd-sysusers`, `systemd-tmpfiles`, `runuser` | Service and installer | OS tools and services, invoked as separate programs. Talking to netlink directly from Rust is a possible later improvement, not a requirement |
| Display server client libraries (`libwayland-client`; Xlib/xcb) | Desktop app (N4) | Loaded at runtime from the system |
| Keyboard layouts (`libxkbcommon`) | Desktop app (N4) | Loaded at runtime from the system |
| Graphics drivers (OpenGL/EGL, Mesa) | Desktop app (N4) | Loaded at runtime from the system |
| D-Bus services: tray host (StatusNotifierWatcher), notifications, xdg-desktop-portal, AT-SPI accessibility bus | Desktop app (N4) | Separate processes, reached through the D-Bus protocol implemented in Rust (`zbus`) |

Evidence for the desktop stack: [rust-gui-evaluation.md](../reviews/rust-gui-evaluation.md).

## 3. Open platform questions (no exception exists)

The M1 feasibility notes ([platform-feasibility.md](platform-feasibility.md)) assumed small non-Rust wrappers on some platforms. **Under D-005 that is not allowed.** These are the precise limitations to resolve **before each platform's work order**. If one cannot be met in Rust, the agent stops and presents it to the owner.

| Platform | Limitation | Rust-only options to verify | Status |
| --- | --- | --- | --- |
| Android | The OS starts a VPN only through a class that **extends `android.net.VpnService`**, declared by name in the app manifest and loaded from the app's own Java bytecode (dex). The OS provides no native-code VPN service. (The UI can use the OS's `NativeActivity` with no Java.) | (a) Generate the required class bytecode from a **Rust** build tool, so no Java or Kotlin source exists; (b) any other way of providing the class that keeps all Septuvo-written code in Rust | **Open.** Verify before the Android work order |
| macOS, iOS, iPadOS | A Network Extension packet tunnel provider is an **Objective-C class** (a subclass of `NEPacketTunnelProvider`) that the OS instantiates **by name** from the extension's `Info.plist` | Define the class **from Rust** with the `objc2` crate's class-definition support (Rust source, using the OS's Objective-C runtime API). Verify that it is registered before the system looks it up | **Open.** Verify before the Apple work orders |
| Windows | Windows has **no built-in layer 3 TUN** for third-party VPNs. The usual solution, Wintun, is a third-party C kernel driver that the app would bundle | (a) Treat a signed third-party network driver as a hardware/OS driver interface (**owner decision**: is bundling it allowed?); (b) a Septuvo network driver written in Rust (Microsoft's `windows-drivers-rs`), which brings its own signing and review burden; (c) any built-in Windows facility found in verification | **Open.** **Owner decision** needed before the Windows work order |
| SVP cryptography | Rust implementations are the first choice | A non-Rust component only via the D-005 SVP route (qualified reviewer, audited, narrowly bounded, owner approval) | Not yet relevant: no cryptographic code before G1 |
