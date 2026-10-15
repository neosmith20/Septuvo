# Septuvo — platform feasibility (M1-04)

**Status:** planning record. **Nothing here is a support claim.** A platform becomes supported only when the OS-integrated tests in [TEST_MATRIX.md](../TEST_MATRIX.md) pass on it (T2, T3) and the owner approves. Entries say **Known** (an established, documented OS facility), **Verify** (believed, but must be checked on real systems) or **Unknown**.
**Order (D-014):** Linux lab first, then Windows → macOS → Android → iOS.
**Rust policy (D-005):** all Septuvo-owned security logic is Rust. Each platform's required non-Rust glue and third-party OS components are listed here as the start of the D-005 inventory.

## 1. Summary

| Platform | Role in version 1 | Tunnel interface | Kill-switch mechanism | Non-Rust glue needed | Overall |
| --- | --- | --- | --- | --- | --- |
| Linux x86_64 (Ubuntu Server 24.04, D-026) | **Server and first client** | TUN (`/dev/net/tun`) — Known | nftables rules — Known; proof in N1-03 | None expected | Feasible; lab target |
| Linux ARM64 (for example a Raspberry Pi home server) | Server, if the owner wants it (Q11) | As above — Known | As above — Known | None expected | Unknown until built and tested |
| Windows 10/11 | Client, after Linux | Needs a signed TUN driver, not written by Septuvo (for example Wintun) — Verify licence and inventory | Windows Filtering Platform — Known facility; leak behaviour Verify | Possibly none beyond OS API bindings; a third-party driver binary | Unknown |
| macOS | Client | Network Extension packet tunnel provider — Known | OS "include all networks" style settings — Verify; known edge cases with connections that existed before the tunnel | Swift or Objective-C extension wrapper around the Rust core; Apple developer account and Network Extension entitlement | Unknown |
| Android | Client | `VpnService` — Known | System "Always-on VPN" + "Block connections without VPN" — Known facility; app-level control Verify | Kotlin/Java service wrapper; Rust via the NDK | Unknown |
| iOS / iPadOS | Client | Network Extension packet tunnel provider — Known | As macOS — Verify | Swift wrapper; strict memory limit for the extension (exact limit Verify) | Unknown |

## 2. Per platform

### 2.1 Linux (server and first client)

- **Interfaces:** TUN through `/dev/net/tun`; policy routing (`ip rule`, separate routing tables); nftables for the kill switch and gateway authorization (network-policies.md NP-E1, NP-G1).
- **DNS:** systemd-resolved per-link DNS on Ubuntu (Verify that NP-D1 and NP-D2 hold with its fallback behaviour); hosts without systemd-resolved are **Unknown**.
- **Privileges:** the node service needs `CAP_NET_ADMIN`. It runs as a system service; the UI or CLI stays unprivileged (TB-1).
- **Key storage:** a root-only or service-only file (KS-1). TPM 2.0 sealing is **Unknown**: current TPMs are not known to hold post-quantum keys, so at best a TPM-held wrapping key (KS-2).
- **Glue:** none expected; Linux system calls from Rust.
- **Evidence path:** N1-03 (namespaces), M5 (real tunnel), G2 lab.

### 2.2 Windows

- **Interfaces:** Windows has no built-in layer 3 TUN for third-party VPNs. An existing **signed** driver is needed; Septuvo writes no drivers in the first phase (ARCHITECTURE.md). Candidate: Wintun, a third-party component, licence and distribution terms to be reviewed under D-015.
- **Kill switch and DNS:** Windows Filtering Platform filters; DNS through interface settings or the Name Resolution Policy Table. Leak behaviour across sleep, network change and service crash: **Verify**.
- **Key storage:** DPAPI (software) or TPM through CNG. Post-quantum keys in the TPM are **Unknown**, so expect KS-2 wrapping.
- **Glue and inventory:** Windows API bindings from Rust; third-party driver binary; code signing for the service and installer (S-12).

### 2.3 macOS

- **Interfaces:** Network Extension framework, packet tunnel provider. Needs an Apple developer account, the Network Extension entitlement, and distribution as a system extension or through the Mac App Store.
- **Kill switch:** OS options that force all traffic into the tunnel exist. VPN vendors have reported cases where connections opened before the tunnel can bypass it. Septuvo must **test**, not assume, NP-E1 and NP-E2 here.
- **Key storage:** Keychain (software). Secure Enclave holds only P-256 keys, so post-quantum and X25519 identities need KS-2 wrapping.
- **Glue:** a Swift or Objective-C extension wrapper calling the Rust core through a C ABI; listed in the D-005 inventory.

### 2.4 Android

- **Interfaces:** `VpnService` gives a TUN-like file descriptor to the app.
- **Kill switch:** the system "Always-on VPN" and "Block connections without VPN" settings are set by the user. The app can explain them but may not be able to force them (**Verify**).
- **Key storage:** Android Keystore and StrongBox, with algorithm support that varies by device and OS version. Post-quantum keys: **Unknown** (expect KS-2 wrapping).
- **Glue:** a Kotlin/Java `VpnService` and UI wrapper; Rust core through the NDK/JNI.

### 2.5 iOS and iPadOS

- **Interfaces:** Network Extension packet tunnel provider, as on macOS, with a strict memory limit for the extension process (exact current limit: **Verify**). The Rust core must fit within it.
- **Kill switch:** as macOS (**Verify**).
- **Key storage:** Keychain; Secure Enclave limited to P-256, so KS-2 wrapping.
- **Glue:** Swift wrapper; App Store review and distribution rules apply.

## 3. Cross-platform facts and exclusions

- **"100% Rust" is not claimed.** Each mobile and Apple platform needs a small native wrapper, and Windows needs a third-party driver; all are inventoried here (D-005).
- **Hardware-backed identity keys are not claimed** on any platform until verified; see KS-2 in [identity-and-privacy.md](identity-and-privacy.md).
- **No custom kernel drivers** in the first phase (ARCHITECTURE.md).
- **Per-application split tunnelling** is not version 1, even where the OS offers it.
- **Architectures:** x86_64 first. ARM64 for the server is Q11; ARM64 clients follow their platform.

## 4. Owner question (this document)

Full explanations, options and trade-offs: [M1-owner-questions.md](../reviews/M1-owner-questions.md). **Decided 2026-10-15: every recommendation below was adopted (D-032).**

| Q | Question | Decision (as recommended) |
| --- | --- | --- |
| Q11 | Should version 1 Septuvo Server also support Linux ARM64 (for example a Raspberry Pi as the home server)? | Yes as a **target**, built and tested in CI on ARM64 once M5 runs, but marked unsupported until its lab tests pass. x86_64 stays first. |
