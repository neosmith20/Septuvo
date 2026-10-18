# Rust inventory and open platform questions (D-005)

**The rule (D-005, owner decided 2026-10-18):**
- **All Septuvo-authored Septuvo VPN and Septuvo Server software is written in Rust.** That covers every client, user interface, service, server-administration tool, installer and piece of platform-specific application code. Septuvo developers do not write VPN or Server code in C, C++, Java, Kotlin, Swift, Objective-C, Python, shell or another language.
- **The implementation language of external components Septuvo calls or interacts with is not restricted.** Operating-system components and APIs, system libraries, third-party libraries, drivers and their user-space DLLs may be written in any language, and may run inside Septuvo's process.
- **Every external component is still reviewed** (security, maintenance, dependency D-015, licensing D-033) and approved by the owner before adoption. The rule authorizes none of them.
- **Septuvo Protocol (SVP): Rust preferred, including its cryptography.** A non-Rust cryptographic implementation may only be *presented* to the owner after a qualified security reviewer finds it necessary or significantly safer, and needs the owner's explicit approval before implementation.

This file is how the rule is checked. It records:
- every file in the repository that Septuvo wrote in a language other than Rust, and its status;
- the external non-Rust components Septuvo uses, and where each was reviewed;
- the places where a future platform may seem to need Septuvo-authored non-Rust code. **Those are open questions, not exceptions.**

## 1. Septuvo-authored files that are not Rust

| Path | What | Shipped with Septuvo VPN or Server? | Status under D-005 |
| --- | --- | --- | --- |
| `packaging/linux/install.sh`, `uninstall.sh` | POSIX shell installer (N3-01) | **Yes** | **Must be replaced by a Rust installer.** Scheduled in N4-07; the shell scripts are removed when it lands |
| `packaging/linux/*.service`, `*.sysusers`, `*.tmpfiles`, `config/*.toml` | systemd unit, sysusers and tmpfiles files; configuration templates | Yes | **Data for the OS service manager and Septuvo's parsers, not code.** Allowed |
| `scripts/*.py`, `scripts/tests/*.py` | Repository checks (ledger, licensing, links) | No: development and CI tooling | Not part of Septuvo VPN or Server. **Owner to confirm** whether to keep or migrate to Rust |
| `tests/lab/**/*.sh`, `tests/lab/namespaces/probe.py` | Lab test harnesses (namespaces, installer) | No: test tooling | As above |
| `fuzz/ci-run.sh` | Fuzzing runner script | No: development only | As above |
| `.github/workflows/*.yml` | CI configuration | No | CI configuration, not code |

**Septuvo-authored product code** (`crates/`, and the coming `desktop/`) **is 100% Rust.**

## 2. External non-Rust components (any language; each reviewed)

| Component | Used by | Language | Review |
| --- | --- | --- | --- |
| Linux kernel (system calls, TUN, netlink, namespaces) | Everything | C | Operating system |
| System C library (`libc`, `libm`, `libgcc_s`, loader) | **Every Rust binary on Linux**, `septuvod` included | C | Operating system |
| `nft`, `ip`, `systemd`, `systemd-sysusers`, `systemd-tmpfiles`, `runuser` | Service and installer | C | Operating-system tools, run as separate programs |
| Display server client libraries (`libwayland-client`; Xlib/xcb), keyboard layouts (`libxkbcommon`), OpenGL/EGL driver (Mesa or the GPU vendor) | Desktop app (N4) | C | Loaded at run time from the system; listed in N4-01's allowlist and review |
| D-Bus services: tray host, notifications, xdg-desktop-portal, AT-SPI accessibility bus | Desktop app (N4) | Various | Separate processes, reached over D-Bus |
| libFuzzer (inside `libfuzzer-sys`) | Fuzzing only (D-035) | C++ | [dependency-review-cargo-fuzz.md](../reviews/dependency-review-cargo-fuzz.md); never shipped |

Evidence for the desktop stack: [rust-gui-evaluation.md](../reviews/rust-gui-evaluation.md). From N4-01, the desktop app's CI gate fails on any non-Rust native component that is not in its reviewed allowlist.

## 3. Open platform questions (no exception exists)

The M1 feasibility notes ([platform-feasibility.md](platform-feasibility.md)) assumed small **Septuvo-written** wrappers in other languages on some platforms. **Under D-005 that is not allowed.** Using external components written in other languages is allowed after review. These are the precise points to resolve **before each platform's work order**. If one cannot be met with Septuvo-authored Rust, the agent stops and presents it to the owner.

| Platform | Limitation | Options to verify | Status |
| --- | --- | --- | --- |
| Android | The OS starts a VPN only through a class that **extends `android.net.VpnService`**, declared by name in the app manifest and loaded from the app's own Java bytecode (dex). The OS provides no native-code VPN service. (The UI can use the OS's `NativeActivity` with no Java.) | (a) Generate the required class bytecode from a **Rust** build tool, so Septuvo writes no Java or Kotlin; (b) a reviewed, independently maintained **external library** that provides the class and calls into Septuvo's Rust code; (c) any other way that keeps all Septuvo-authored code in Rust | **Open.** Verify before the Android work order |
| macOS, iOS, iPadOS | A Network Extension packet tunnel provider is an **Objective-C class** (a subclass of `NEPacketTunnelProvider`) that the OS instantiates **by name** from the extension's `Info.plist` | Define the class **from Rust** with the `objc2` crate's class-definition support. Verify that it is registered before the system looks it up. A reviewed external component is the alternative | **Open.** Verify before the Apple work orders |
| Windows | Windows has **no built-in layer 3 TUN device** for third-party VPNs | **D-036:** an existing, signed third-party driver used through its own DLL (for example Wintun) is acceptable in principle; Septuvo's calling code is Rust. Options compared in [windows-tunnel-evaluation.md](../reviews/windows-tunnel-evaluation.md): Wintun first, Microsoft's VPN plug-in platform as the alternative. No custom driver unless necessary | **Rule set (D-036); choice open.** Review and owner approval needed. Also open: Septuvo's no-`unsafe` rule versus foreign-function calls |
| SVP cryptography | Rust implementations are the first choice | A non-Rust implementation only via the D-005 SVP route (qualified reviewer, audited, narrowly bounded, owner approval) | Not yet relevant: no cryptographic code before G1 |
