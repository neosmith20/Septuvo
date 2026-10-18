# Windows tunnel interface evaluation (D-005, D-036)

**Prepared 2026-10-18, revised the same day after the owner clarified D-005.** This is a **planning record for a future Windows work order**. It does not block N4, and it **approves nothing**:
- no driver, DLL or Windows dependency is adopted;
- each needs a security, maintenance, dependency and licence review, and the owner's explicit approval (D-036).

## The question

Windows has **no built-in layer 3 tunnel device** (like Linux's `/dev/net/tun`) for third-party VPNs. Septuvo VPN on Windows needs some way to get IP packets in and out of the OS network stack.

The owner's rule (D-005 and D-036, 2026-10-18):
- **Septuvo-authored code is Rust.** The language of external components Septuvo calls is not restricted.
- **An existing, suitable, signed driver is acceptable in principle**, including through its own user-space DLL **loaded into Septuvo's process** (for example Wintun's `wintun.dll`).
- **Septuvo does not have to write its own driver** when an existing one can do the job. No custom kernel driver without compelling necessity.
- **Nothing is approved by default**, Wintun included.

## Method

The candidates' published documentation, licences and repositories were read (sources at the end). **Nothing was built or run on Windows.** Every "Verify" item below needs testing on real Windows 10/11 machines in the Windows work order.

## Options compared

| | A. Wintun | B. Microsoft VPN plug-in platform | C. Another signed driver | D. A Septuvo driver written in Rust |
| --- | --- | --- | --- | --- |
| **What it is** | WireGuard LLC's layer 3 TUN driver, used through `wintun.dll` (C), which installs the driver and moves packets through a shared ring buffer | Windows' own VPN framework (`Windows.Networking.Vpn`): the OS creates the layer 3 interface and calls the app's plug-in for each batch of packets | An existing signed driver used from Rust through its own interface | Septuvo's own NetAdapterCx driver, using Microsoft's `windows-drivers-rs` |
| **Septuvo-authored code** | Rust only, calling the DLL's published API | Rust only, through Microsoft's `windows` crate | Rust only | Rust, including kernel code |
| **Fits D-005 / D-036** | **Yes, in principle** (the DLL is an external component) | Yes | Yes, if the driver is suitable | Only if A–C fail (D-036) |
| **Licence** | The signed binaries from wintun.net are under WireGuard LLC's prebuilt-binaries licence (source: GPL-2.0, not used). As published for WireGuard-NT, it allows use and redistribution **alongside software that uses it only through its published API**, forbids modification and reverse engineering, and requires notices to stay. Septuvo would ship the unmodified signed DLL and call its API. **Verify** the exact text in the Wintun ZIP during review | Part of Windows; nothing to license | Depends on the driver (below) | Septuvo's own (D-033) |
| **Packaging and distribution** | One DLL next to the app; the DLL installs its driver. Admin rights needed to create adapters. "Do not distribute files named Wintun" other than the official ones | **A packaged (MSIX) app** with the **restricted capability `networkingVpnProvider`**; Microsoft says VPN apps are curated, which means Store or curation approval (**Verify** for sideloaded and enterprise installs) | Installer adds the vendor's signed driver | **EV code-signing certificate plus Microsoft attestation signing** for every release; a kernel driver to maintain |
| **Security** | Mature, widely deployed (WireGuard for Windows and others). Runs in Septuvo's privileged service process, so a DLL flaw would be a service flaw: covered by the review, pinned version and checksum | The plug-in runs **sandboxed** (least privilege). Septuvo's kill switch (Windows Filtering Platform) and DNS policy need a separate privileged service; how the two interact must be designed and tested (**Verify**). Cisco documents a DNS limitation affecting some plug-in VPNs | As good as the chosen driver | **Kernel code is the highest-risk code Septuvo could write**: a bug can crash or compromise the machine |
| **Performance** | High (shared ring buffer) | Packets cross the WinRT boundary through Encapsulate/Decapsulate callbacks; **throughput unknown, must be measured** | Depends on the driver | Potentially high, after a lot of work |
| **Maintenance signals** | Active upstream; latest release 0.14.1 | Windows 10 and 11 API, still documented. **Microsoft's official sample was archived on 2026-06-11**, a signal to watch | See below | Entirely Septuvo's burden; `windows-drivers-rs` is young (cargo-wdk 0.1.x) |
| **Verdict** | **First candidate for review and prototyping** | **Alternative** if Wintun fails review, or to compare | No better candidate found (below) | **Last resort** only |

### Other drivers checked (option C)

| Driver | Licence | Fit |
| --- | --- | --- |
| TAP-Windows6 (OpenVPN) | GPL-2.0 and MIT files in the repository | **Legacy NDIS 6 Ethernet (layer 2) driver**; Septuvo is layer 3 (D-003). No published releases on its repository. Weak fit |
| ovpn-dco-win (OpenVPN) | MIT | **Not a generic tunnel.** It performs OpenVPN's own data-channel encryption in the kernel, so it cannot carry SVP |
| WireGuard-NT | As Wintun (prebuilt-binaries licence) | WireGuard-specific; cannot carry SVP |

## Recommendation (for the Windows work order, not now)

1. **Review Wintun** as a D-015 dependency: exact version and checksum of the signed DLL, the licence text in its ZIP, its security history, its maintenance, how the DLL is loaded (a fixed path next to the binary, never from a search path), and admin-rights behaviour.
2. **Prototype it from Rust** (Septuvo's own bindings, or a reviewed binding crate) and measure throughput, latency, and the kill switch's fail-closed behaviour across sleep, network change and crashes (NP-E1, NP-E2).
3. **Compare with Microsoft's VPN plug-in platform** where it matters (capability approval, packaging, DNS behaviour), and bring the choice to the owner. **No driver or DLL is adopted without explicit approval.**

## A second owner decision the Windows work will need

**Septuvo forbids `unsafe` Rust.** Calling a DLL or most Windows APIs from Rust goes through foreign-function calls, which Rust marks `unsafe`. A Windows build will therefore need the owner to decide between:
- (a) a narrowly scoped, reviewed exception for small OS- and DLL-binding modules;
- (b) relying only on safe wrapper crates maintained by others (themselves reviewed dependencies).

This is recorded here so it is not discovered late; **nothing is decided**.

## Sources

- Wintun: [wintun.net](https://www.wintun.net/), [project README](https://git.zx2c4.com/wintun/about/), [Wintun 0.9 announcement](https://lists.zx2c4.com/pipermail/wireguard/2020-November/005999.html)
- WireGuard LLC prebuilt-binaries licence (WireGuard-NT copy): [prebuilt-binaries-license.txt](https://git.zx2c4.com/wireguard-nt/tree/prebuilt-binaries-license.txt)
- Microsoft VPN plug-in platform: [IVpnPlugIn](https://learn.microsoft.com/en-us/uwp/api/windows.networking.vpn.ivpnplugin?view=winrt-26100), [VpnChannel](https://learn.microsoft.com/en-us/uwp/api/windows.networking.vpn.vpnchannel?view=winrt-26100), [ProcessEventAsync](https://learn.microsoft.com/en-us/uwp/api/windows.networking.vpn.ivpnchannelstatics.processeventasync?view=winrt-26100), [Microsoft's VPN plug-in sample (archived)](https://github.com/microsoft/uwpvpnpluginsample)
- DNS limitation of plug-in VPNs: [Cisco tech note 224662](https://www.cisco.com/c/en/us/support/docs/security/umbrella/224662-windows-10-native-vpn-api-modern-metro.html)
- Other drivers: [TAP-Windows6](https://github.com/OpenVPN/tap-windows6), [ovpn-dco-win](https://github.com/OpenVPN/ovpn-dco-win)
- Rust drivers: [windows-drivers-rs](https://github.com/microsoft/windows-drivers-rs); driver signing: [Microsoft driver signing offerings](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings)
