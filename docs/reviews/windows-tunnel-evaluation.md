# Windows tunnel interface evaluation (D-005, D-036)

**Prepared 2026-10-18.** This is a **planning record for a future Windows work order**. It does not block N4, and it **approves nothing**:
- no driver, DLL or Windows dependency is adopted;
- each needs the owner's explicit approval (D-036).

## The question

Windows has **no built-in layer 3 tunnel device** (like Linux's `/dev/net/tun`) for third-party VPNs. Septuvo VPN on Windows needs some way to get IP packets in and out of the OS network stack.

The owner's rule for this (D-036, clarifying D-005):
- **Allowed in principle:** an **independently maintained, signed Windows network driver**, as an external operating-system component. Windows itself, its kernel and third-party OS drivers do not have to be Rust.
- **Not allowed:** non-Rust executable code inside Septuvo's own processes, or bundled as a Septuvo application-level component.
- **No custom Windows kernel driver** without a compelling technical necessity.
- **Nothing is approved by default.** That includes Wintun.

## Method

The candidates' published documentation, licences and repositories were read (sources at the end). **Nothing was built or run on Windows.** Every "Verify" item below needs testing on real Windows 10/11 machines in the Windows work order.

## Options compared

| | A. Wintun | B. Microsoft VPN plug-in platform | C. A signed driver used directly from Rust (no vendor DLL) | D. A Septuvo driver written in Rust |
| --- | --- | --- | --- | --- |
| **What it is** | WireGuard LLC's layer 3 TUN driver, used through `wintun.dll` | Windows' own VPN framework (`Windows.Networking.Vpn`): the OS creates the layer 3 interface and calls the app's plug-in for each batch of packets | An existing signed driver whose user-space interface is plain `CreateFile` / `DeviceIoControl` / `ReadFile` / `WriteFile`, called from Rust | Septuvo's own NetAdapterCx driver, using Microsoft's `windows-drivers-rs` |
| **Non-Rust code in Septuvo's processes** | **Yes.** `wintun.dll` (C) is loaded into the app's process (`LoadLibraryEx`) and runs there. wintun.net says the signed DLLs are "the only supported way of distributing Wintun", and the DLL installs the driver | **None.** Windows' own components (an OS interface); Septuvo's plug-in is Rust through Microsoft's `windows` crate | **None.** The driver is an external OS component; Septuvo only makes system calls | None in user space. The driver itself is Rust, but needs large FFI bindings to the WDK |
| **Compatible with D-005 / D-036 as is** | **No** (the DLL). Moving it to a helper process does not help: it is still bundled non-Rust application code | **Yes**, subject to the points below | **Yes**, if the owner approves the specific driver | Yes, but D-036 says only if A–C fail |
| **Licence** | Source GPL-2.0. Signed binaries are under WireGuard LLC's prebuilt-binaries licence, which (as published for WireGuard-NT) forbids modification and reverse engineering. It allows redistribution only alongside software that uses the published API, which is the DLL's API. **Verify** the copy in the Wintun ZIP | Part of Windows; nothing to license | Depends on the driver (below) | Septuvo's own (D-033) |
| **Could Rust replace the DLL?** | **Not lawfully in practice.** A port of the GPL-2.0 user-space code would be GPL-2.0 (conflicts with D-033). Reverse engineering the binaries is forbidden. The project also warns against distributing files named "Wintun" | — | — | — |
| **Packaging and distribution** | A DLL next to the app | **A packaged (MSIX) app** with the **restricted capability `networkingVpnProvider`**. Microsoft says VPN apps are curated, so this means Store or curation approval (**Verify** for sideloaded and enterprise installs). Profiles are managed through Windows' VPN settings and MDM | Septuvo's installer installs the vendor's signed driver | **EV code-signing certificate plus Microsoft attestation signing** for every release; a kernel driver to maintain |
| **Security** | Mature and widely deployed | The plug-in runs **sandboxed** in the app container (least privilege). But Septuvo's kill switch (Windows Filtering Platform) and DNS policy need a separate privileged service; how the two interact must be designed and tested (**Verify**). Cisco documents a DNS limitation affecting some plug-in VPNs | As good as the chosen driver | **Kernel code is the highest-risk code Septuvo could write**: a bug can crash or compromise the whole machine. It would need its own expert review |
| **Performance** | High (shared ring buffer) | Packets cross the WinRT boundary through Encapsulate/Decapsulate callbacks; **throughput unknown, must be measured** | Depends on the driver | Potentially high, after a lot of work |
| **Maintenance signals** | Active upstream; widely used | Windows 10 and 11 API, still documented. **Microsoft's official sample was archived on 2026-06-11**, a signal to watch, not a deprecation | See below | Entirely Septuvo's burden; `windows-drivers-rs` is young (cargo-wdk 0.1.x) |
| **Verdict** | **Not compatible** without an owner-approved exception. Not recommended | **Recommended to prototype first**: the only option with no third-party native code at all | **Fallback**, if B fails and a suitable driver exists | **Last resort** only |

### Drivers checked for option C

| Driver | Licence | Interface | Fit |
| --- | --- | --- | --- |
| TAP-Windows6 (OpenVPN) | GPL-2.0 and MIT files in the repository | Device file and IOCTLs (header `tap-windows.h`); no DLL | **Legacy NDIS 6 Ethernet (layer 2) driver**; any layer 3 mode needs checking (**Verify**). No published releases on its repository; signed builds ship inside OpenVPN's installers (**Verify** the redistribution terms). Septuvo is layer 3 (D-003), so this is a weak fit |
| ovpn-dco-win (OpenVPN) | MIT | Device file and IOCTLs; no DLL | **Not a generic tunnel.** It performs OpenVPN's own data-channel encryption in the kernel, so it cannot carry SVP. Rejected |
| WireGuard-NT | As Wintun (prebuilt-binaries licence) | DLL | WireGuard-specific and DLL-based. Rejected (SVP is not WireGuard) |

**No generic, signed, layer 3 driver with a DLL-free interface was found.** Option C currently has no good candidate. That is why B comes first, and D stays a documented last resort.

## Recommendation (for the Windows work order, not now)

1. **Prototype B, the Microsoft VPN plug-in platform, in Rust**, through Microsoft's `windows` crate (a Rust crate generated from Windows' own API metadata; an OS interface). Measure and verify:
   - throughput and latency through Encapsulate/Decapsulate;
   - whether the `networkingVpnProvider` capability can be obtained, and what it requires for Store, sideloaded and enterprise installs;
   - DNS behaviour (including the documented limitation);
   - how a separate privileged Septuvo service enforces the kill switch (Windows Filtering Platform) and fail-closed behaviour alongside the plug-in, across sleep, network change and crashes (NP-E1, NP-E2);
   - Windows 10 and 11 support.
2. **If B fails** on performance, capability or kill-switch grounds, bring the findings to the owner with the C and D options. **Do not adopt a driver without explicit approval.**
3. **Wintun is not proposed.** It would need an owner-approved exception to D-005 and D-036, and its licence prevents a Rust replacement for the DLL.

## A second owner decision the Windows work will need

**Septuvo forbids `unsafe` Rust.** Most Windows API calls through the `windows` crate are `unsafe` functions, because they are foreign-function calls. A Windows build will therefore need the owner to decide between:
- (a) a narrowly scoped, reviewed exception for small OS-binding modules;
- (b) relying only on safe wrappers maintained by others.

This is recorded here so it is not discovered late; **nothing is decided**.

## Sources

- Wintun: [wintun.net](https://www.wintun.net/), [project README](https://git.zx2c4.com/wintun/about/), [Wintun 0.9 announcement](https://lists.zx2c4.com/pipermail/wireguard/2020-November/005999.html)
- WireGuard LLC prebuilt-binaries licence (WireGuard-NT copy): [prebuilt-binaries-license.txt](https://git.zx2c4.com/wireguard-nt/tree/prebuilt-binaries-license.txt)
- Microsoft VPN plug-in platform: [IVpnPlugIn](https://learn.microsoft.com/en-us/uwp/api/windows.networking.vpn.ivpnplugin?view=winrt-26100), [VpnChannel](https://learn.microsoft.com/en-us/uwp/api/windows.networking.vpn.vpnchannel?view=winrt-26100), [ProcessEventAsync](https://learn.microsoft.com/en-us/uwp/api/windows.networking.vpn.ivpnchannelstatics.processeventasync?view=winrt-26100), [Microsoft's VPN plug-in sample (archived)](https://github.com/microsoft/uwpvpnpluginsample)
- DNS limitation of plug-in VPNs: [Cisco tech note 224662](https://www.cisco.com/c/en/us/support/docs/security/umbrella/224662-windows-10-native-vpn-api-modern-metro.html)
- Other drivers: [TAP-Windows6](https://github.com/OpenVPN/tap-windows6), [ovpn-dco-win](https://github.com/OpenVPN/ovpn-dco-win)
- Rust drivers: [windows-drivers-rs](https://github.com/microsoft/windows-drivers-rs); driver signing: [Microsoft driver signing offerings](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings)
