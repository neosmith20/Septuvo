# Septuvo — threats, trust boundaries and assumptions (M1-01)

**Status:** requirements draft for owner review, pending qualified outside review at G1. It is not a security proof. It chooses **no** cryptography: D-010, D-011, D-012 and D-024 stay candidates and D-013 stays blocked.
**Scope:** version 1 (D-027): **journey A**, reach the home network from anywhere, and **journey B**, protect traffic on untrusted Wi-Fi, both through the user's own Septuvo Server. Organization and provider deployments (D-009) are listed only where they change a requirement; they are not version 1.
**Baseline:** the attacker and out-of-scope lists in [SECURITY_MODEL.md](../SECURITY_MODEL.md) (D-028), refined here.

## 1. System in version 1

| Part | What it is | Runs where |
| --- | --- | --- |
| Septuvo VPN app | Unprivileged user interface (desktop app or CLI) | User's device |
| Septuvo node service | Privileged service: identity, SVP sessions, policy, routes, firewall, DNS | User's device |
| Septuvo Server | The user's own gateway: authenticates devices, forwards to the home network (journey A) or the internet (journey B) | Home server or a rented VPS, Linux (D-026) |
| Home network | Devices behind the server such as a NAS or media server | User's home |
| Underlay network | Whatever network the device is attached to: café or hotel Wi-Fi, mobile data, the home LAN | Anywhere |
| Release channel | Source repository, CI, signed packages and updates | GitHub and distribution sites |

There is no Septuvo cloud, account or relay in version 1 (D-007).

## 2. Trust boundaries

| ID | Boundary | What crosses it | Who is on the far side |
| --- | --- | --- | --- |
| TB-1 | App ↔ node service on the same device | Commands, status, policy changes | Any local process, which may be malicious (ADV-10) |
| TB-2 | Device ↔ underlay network | All packets, DHCP, router advertisements, DNS | The local network operator and anyone on it (ADV-01) |
| TB-3 | Device ↔ Septuvo Server over the internet | SVP handshake and encrypted traffic | Every network in between (ADV-02, ADV-03, ADV-04) |
| TB-4 | Server ↔ home network | Forwarded traffic of authenticated devices | Home devices, which are not assumed secure |
| TB-5 | Server ↔ internet (exit, journey B) | Decrypted user traffic, after NAT | Destination sites and the server's upstream provider |
| TB-6 | Enrollment channel | Invitation (QR code or link) | Whoever can see or intercept it (ADV-06) |
| TB-7 | Administrator ↔ server | Device list, invitations, revocation, policy | The server host's local administrator |
| TB-8 | Release channel ↔ device or server | Source, builds, packages, updates | Build systems, dependencies, accounts (ADV-09) |
| TB-9 | Septuvo ↔ operating system | Routes, firewall, DNS, key storage, TUN | The OS, trusted (ASM-01) but with platform limits ([platform-feasibility.md](platform-feasibility.md)) |

## 3. Assets

| ID | Asset | Why it matters |
| --- | --- | --- |
| AST-01 | Device and server long-term identity private keys | Whoever holds one can impersonate that device or server |
| AST-02 | Session secrets and traffic keys | Decrypt or forge traffic of a session |
| AST-03 | Unused invitations | Bearer secrets: whoever uses one first becomes an enrolled device |
| AST-04 | User traffic plaintext | The thing a VPN protects |
| AST-05 | DNS queries and answers | Reveal and steer what the user visits |
| AST-06 | Traffic metadata: endpoints, timing, sizes, which device talks to which | Profiles the user; only partly protectable (non-goals) |
| AST-07 | Routing and LAN-exception policy | Decides what is protected; tampering causes leaks |
| AST-08 | Server authorization data: enrolled devices, allowed destinations, revocations | Decides who reaches the home network |
| AST-09 | Home network devices | Often weakly secured; the gateway must not expose them more than intended |
| AST-10 | Logs and diagnostics | May contain metadata, names or secrets if done carelessly |
| AST-11 | Release signing keys and the integrity of updates | A tampered update owns every device |
| AST-12 | Privileged network configuration of the host | Wrong or stale rules leak traffic or cut off the user |

## 4. Adversaries

ADV-01 to ADV-06 and ADV-09 come from the D-028 baseline. ADV-07, ADV-08 and ADV-10 are added by this document for owner review (section 7). ADV-11 and ADV-12 are not version 1.

| ID | Adversary | Can | Cannot (assumed) |
| --- | --- | --- | --- |
| ADV-01 | Hostile local network: rogue or compromised Wi-Fi, hotel or café operator, another user on the LAN | Read, drop, delay, replay, modify and inject any local packet; run rogue DHCP, router advertisements, DNS and captive portals; pick the local subnet, including one that overlaps the home subnet | Break the selected cryptography; control the device's OS |
| ADV-02 | On-path internet attacker: ISP, transit network, state actor | Everything ADV-01 can, on the path between device and server | As ADV-01 |
| ADV-03 | Internet attacker against the server's open UDP port | Send unlimited forged, malformed, replayed or spoofed-source packets; scan; attempt amplification and resource exhaustion | Exceed the server's network capacity without that being a non-goal (volumetric floods) |
| ADV-04 | Recorder for later quantum cryptanalysis ("harvest now, decrypt later") | Store all encrypted traffic today and attack it with a future quantum computer | Break the post-quantum part of a correctly reviewed hybrid construction |
| ADV-05 | Thief or finder of a device | Take a locked or unlocked device; extract stored data from a powered-off device | Defeat hardware-backed key protection where it exists (platform-dependent) |
| ADV-06 | Holder of an intercepted or leaked invitation | Try to enroll before the intended device; replay the invitation | Use it after it was used, expired or revoked |
| ADV-07 | Malicious or compromised enrolled device (added) | Authenticate legitimately; try to reach destinations it is not allowed to; attack other devices through the server; flood the server | Impersonate a different device or the server |
| ADV-08 | Party controlling the server's host (added): VPS provider, malware on the home server | See exit traffic after decryption; read server keys and authorization data | Nothing: the server is a trusted endpoint, so this is a non-goal. Rented-VPS users must understand the provider can see exit traffic |
| ADV-09 | Supply-chain attacker | Compromise a dependency, CI job, maintainer account or update path | Pass the review and signing controls required at G3 unnoticed (goal, to be proven) |
| ADV-10 | Unprivileged local process or user on the device (added) | Call the node service over TB-1; read world-readable files; spoof the app | Use administrator or root rights (ASM-01) |
| ADV-11 | Malicious tenant on shared provider infrastructure | Attack other tenants through shared gateways | Not version 1; G4 |
| ADV-12 | Compromised administrator credential or control-plane node | Change policy or enroll devices | Not version 1 beyond the local server administrator (TB-7); M9 |

## 5. Assumptions

| ID | Assumption | If it is false |
| --- | --- | --- |
| ASM-01 | The device and server operating systems, kernels and the Septuvo node service are not compromised; administrator/root is trusted | Out of scope: a compromised endpoint sees plaintext |
| ASM-02 | The OS random number generator is sound | Keys may be predictable; nothing in Septuvo can compensate |
| ASM-03 | The primitives selected in M2 and reviewed at G1 are secure as specified, and a correctly reviewed hybrid holds if either component holds | The G1 review and algorithm agility (S-05) are the mitigation |
| ASM-04 | The user can compare the server's identity once, out of band, at enrollment (for example a short code or a QR code shown on the server) | Enrollment falls back to trust on first use; this is a weaker, disclosed mode |
| ASM-05 | Clocks may be wrong by minutes and can be manipulated by the local network (ADV-01) | Expiry and replay rules must not depend on accurate client time alone |
| ASM-06 | Once applied, OS firewall rules are enforced by the kernel even if the Septuvo service crashes | Must be proven per platform (N1-03 on Linux, M5, M7); unproven platforms are not supported |
| ASM-07 | The server operator is the user (personal profile) and is trusted with exit traffic | Organization and provider profiles need separate requirements (M9) |
| ASM-08 | Every underlay network is hostile, including the home LAN (D-006) | — (this is the default, not a hope) |
| ASM-09 | Home network devices may be insecure | The gateway must restrict reachability, not assume device security |

## 6. Security requirements against adversaries

Every requirement in [SECURITY_MODEL.md](../SECURITY_MODEL.md) mapped to the adversaries that motivate it and the assumptions it relies on.

| Requirement | Adversaries | Assumptions | Main boundary | Specified in |
| --- | --- | --- | --- | --- |
| S-01 Authenticated key agreement | ADV-01, ADV-02, ADV-07 | ASM-02, ASM-03, ASM-04 | TB-3 | M2-02 |
| S-02 Quantum resistance goal | ADV-04 | ASM-03 | TB-3 | M2-01, M2-02 |
| S-03 Key lifecycle | ADV-05, ADV-10, ADV-02 | ASM-01, ASM-02 | TB-9, TB-3 | M2-03; storage in [identity-and-privacy.md](identity-and-privacy.md) |
| S-04 AEAD safety | ADV-01, ADV-02 | ASM-03 | TB-3 | M2-03 |
| S-05 Downgrade safety | ADV-01, ADV-02 | ASM-03 | TB-3 | M2-02 |
| S-06 Replay and endpoint changes | ADV-01, ADV-02, ADV-03 | ASM-05 | TB-3 | M2-03 |
| S-07 DoS | ADV-03, ADV-07 | — | TB-3 | M2-03 |
| S-08 Hostile networks, fail closed | ADV-01, ADV-02 | ASM-06, ASM-08 | TB-2, TB-9 | [network-policies.md](network-policies.md) |
| S-09 Authorization | ADV-07 | ASM-09 | TB-4 | [network-policies.md](network-policies.md) section 7 |
| S-10 Enrollment | ADV-06, ADV-01, ADV-05 | ASM-04, ASM-05 | TB-6 | [identity-and-privacy.md](identity-and-privacy.md) |
| S-11 Control plane | ADV-12, ADV-10 | ASM-07 | TB-7, TB-1 | [identity-and-privacy.md](identity-and-privacy.md) section 6; M9 |
| S-12 Updates and dependencies | ADV-09 | ASM-01 | TB-8 | M3-01, M6-02, M8-03 |
| S-13 Diagnostics and privacy | ADV-10, ADV-08, ADV-05 | ASM-01 | TB-1, TB-9 | [identity-and-privacy.md](identity-and-privacy.md) section 7 |

## 7. Newly exposed risks for owner review

| ID | Risk | Effect | Proposed handling |
| --- | --- | --- | --- |
| R-01 | A home-hosted server needs an open UDP port on the home router (port forward) | The home IP and that port face ADV-03 | Accept for version 1, with strict pre-authentication limits (S-07). Relays stay opt-in only (D-007). **Owner decision Q12** |
| R-02 | A server on a rented VPS lets the provider see exit traffic (ADV-08) | Journey B privacy depends on the VPS provider | State it plainly in setup; offer the home server as the private option |
| R-03 | Split mode (journey A) leaves everything except the home network unprotected | Users may believe they are protected on café Wi-Fi | The UI must say "only home traffic is protected" in split mode ([network-policies.md](network-policies.md)) |
| R-04 | The café subnet often equals the home subnet (both `192.168.1.0/24`) | Home traffic could be sent to the café network | Tunnel wins while connected; never fall back to the local network (Q5) |
| R-05 | Name lookups for home devices (`nas.home`) leak to the café DNS in split mode | Reveals home device names; can be spoofed | Home domains always resolve through the tunnel ([network-policies.md](network-policies.md) section 6) |
| R-06 | Hardware key stores often cannot hold post-quantum keys (for example Apple Secure Enclave is limited to P-256) | Identity keys may be software-protected only | Wrap with a hardware key where possible and disclose per platform ([platform-feasibility.md](platform-feasibility.md)) |
| R-07 | A malicious enrolled device (ADV-07) could reach the whole home network or other devices | Lateral movement in the home | Per-device allowed destinations at the gateway; device-to-device traffic off by default (Q6) |
| R-08 | Agents and the owner share one GitHub account (D-023) | A supply-chain mistake could reach releases | Release signing keys never available to agents or CI jobs agents can edit; decided before G3 |
| R-09 | Captive portals (hotel login pages) need traffic outside the tunnel | Users may switch protection off entirely to log in | A deliberate, limited captive-portal mode (Q1) instead of a full bypass |
| R-10 | Traffic before the service starts at boot, or during an update, can leak | Short unprotected windows | Optional always-on mode with boot-time blocking (Q4); updates keep rules in place |

## 8. Non-goals (version 1)

- Protecting a compromised device, server or server host (ASM-01, ADV-08).
- Anonymity: the server operator and destinations see the server's address; the underlay sees that a Septuvo connection exists.
- Hiding traffic timing, sizes or volume (traffic analysis).
- Hiding the use of a VPN, or getting through censorship (obfuscation). Not decided as a future goal; listed as **OPEN** for the owner.
- Availability under volumetric floods beyond the server's network capacity.
- Securing home services that have weak authentication of their own; a LAN exception does not encrypt anything.
- Multi-tenant provider isolation and organization control planes (ADV-11, ADV-12): M9 and G4.
