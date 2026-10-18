# Septuvo — requirement and evidence traceability

This is a **planned** traceability map (baseline from M1-04). No listed test is considered run or passed unless its row says where real results are recorded. Test IDs are proposed identifiers; test code and files are added only under authorized work orders.

**Sources:** security requirements S-01…S-13 in [SECURITY_MODEL.md](SECURITY_MODEL.md); adversaries ADV-* in [spec/threats.md](spec/threats.md); network requirements NP-* in [spec/network-policies.md](spec/network-policies.md); identity and privacy requirements (ID, EN, KS, RV, AD, PR, LO) in [spec/identity-and-privacy.md](spec/identity-and-privacy.md); platform status in [spec/platform-feasibility.md](spec/platform-feasibility.md); test levels T0–T5 in [TEST_MATRIX.md](TEST_MATRIX.md).

## Components

| ID | Component | Exists today? |
| --- | --- | --- |
| C-APP | Unprivileged app and CLI (`crates/septuvo-cli`) | Skeleton (N1-01) |
| C-NODE | Privileged node service: sessions, policy application, local control (TB-1) | No |
| C-POL | Policy engine (`crates/septuvo-policy`) | Yes, pure logic (N1-02) |
| C-OSNET | OS enforcement: routes, firewall (kill switch), DNS | Lab harness planned (N1-03) |
| C-SVP | SVP handshake and session state machine | No — after G1 |
| C-PKT | Packet engine: encryption, replay protection, framing | No — after G1 |
| C-ID | Identity, enrollment, key storage, revocation | No |
| C-GW | Septuvo Server gateway: per-device authorization, NAT, forwarding | No |
| C-REL | Build, release, signing and update | CI checks only |
| C-CP | Organization/provider control plane | No — M9 |

## Security requirements (S-01 through S-13)

| ID | Components | Adversaries | Tasks | Test IDs | Minimum gate |
| --- | --- | --- | --- | --- | --- |
| S-01 authenticated key agreement | C-SVP, C-ID | ADV-01, ADV-02, ADV-07 | M2-02/03, M3-03/04 | T1-HS-01, T1-HS-02 | G1 design; G3 production |
| S-02 hybrid PQ goal | C-SVP | ADV-04 | M2-01/02/05, M3-03 | T1-HS-03, T1-DG-01 | G1 reviewed design; G3 audit |
| S-03 key lifecycle | C-SVP, C-PKT, C-ID | ADV-05, ADV-10, ADV-02 | M2-03, M3-03/04, M4-03 | T1-KL-01, T1-KL-02, T3-KS-01 | G1; G3 |
| S-04 AEAD nonce/replay | C-PKT | ADV-01, ADV-02 | M2-03, M3-04, M4-03 | T1-AE-01, T1-AE-02, T1-KL-02 | G1; G3 |
| S-05 downgrade prevention | C-SVP | ADV-01, ADV-02 | M2-02, M3-04 | T1-DG-01, T1-HS-02 | G1; G3 |
| S-06 session replay/migration | C-PKT, C-SVP | ADV-01, ADV-02, ADV-03 | M2-03, M3-04, M4-03, M7-05 | T1-RP-01, T1-RP-02, T2-KS-05 | G1; G3 |
| S-07 resource/DoS bounds | C-PKT, C-SVP, C-GW | ADV-03, ADV-07 | M2-03, M3-02, M5-04, M9-04 | T1-DOS-01, T1-DOS-02, T5-DOS-01 | G1; G3/G4 |
| S-08 hostile network fail closed | C-OSNET, C-POL, C-NODE | ADV-01, ADV-02 | M1-02, N1-02, N1-03, M5-01/02, M7-01/05 | T2-POL-01, T2-KS-01…05, T2-DNS-01/02, T2-V6-01, T2-OV-01 | G2 lab; G3 per OS |
| S-09 least-privilege access | C-GW, C-POL | ADV-07 | M1-02, M4-02, M5-01/03, M9-04 | T2-GW-01, T2-GW-02, T5-TEN-01 | G2; G4 provider |
| S-10 safe enrollment/revocation | C-ID, C-GW | ADV-06, ADV-01, ADV-05 | M1-03, M6-01 | T3-EN-01, T3-EN-02, T3-RV-01, T3-RV-02 | G3 |
| S-11 control-plane separation | C-NODE, C-GW; C-CP later | ADV-10, ADV-12 | M1-03, M9-01/02/03 | T3-AD-01; T5-CP-01 (M9) | G3 (local); G4 (control plane) |
| S-12 signing/dependency/update | C-REL | ADV-09 | M3-01/03, M6-02, M7-03, M8-03 | T0-SC-01, T3-PKG-01, T3-REL-01 | G3 |
| S-13 privacy/diagnostics | C-APP, C-NODE, C-GW | ADV-10, ADV-08, ADV-05 | M1-03, N1-04, M6-03, M7-02 | T3-ST-01, T3-LOG-01, T3-OFF-01 | G3 |

Every S requirement has at least one adversary, component, test ID and gate: **no orphan requirement**.

## Test ID catalogue

| Test ID | Level | What it must show | Earliest task | Real results |
| --- | --- | --- | --- | --- |
| T0-SC-01 | T0 | CI: secret scanning, actions pinned by SHA, no unapproved crates, integrity checks | N1-01, H-007 | `repository-integrity` CI on every pull request; `Rust` CI once N1-01 is merged |
| T1-HS-01 | T1 | Wrong or unknown peer identity and forged authentication are rejected | M3-03 | NOT RUN |
| T1-HS-02 | T1 | Transcript, role or version tampering is rejected | M3-03 | NOT RUN |
| T1-HS-03 | T1 | Hybrid key establishment cases specified by the G1 reviewer | M3-03 | NOT RUN |
| T1-DG-01 | T1 | Version, suite and post-quantum-required downgrades fail closed | M3-04 | NOT RUN |
| T1-KL-01 | T1 | Key separation, expiry, rekey under loss, stale keys rejected | M3-04 | NOT RUN |
| T1-KL-02 | T1 | Crash and restart never reuse a key/nonce pair | M4-03 | NOT RUN |
| T1-AE-01 | T1 | Nonce uniqueness across workers and epochs; counter rollover | M4-03 | NOT RUN |
| T1-AE-02 | T1 | Corrupt tag, ciphertext or length rejected | M3-04 | NOT RUN |
| T1-RP-01 | T1 | Duplicates and old packets dropped by the replay window | M3-04 | NOT RUN |
| T1-RP-02 | T1 | Authenticated endpoint change accepted; spoofed migration rejected | M4-03 | NOT RUN |
| T1-DOS-01 | T1 | Malformed, oversized and fuzzed input: no panic, bounded memory | M3-02 | NOT RUN |
| T1-DOS-02 | T1 | Pre-authentication amplification and rate limits within bounds | M3-04 | NOT RUN |
| T2-POL-01 | T2 | Policy decisions for journeys A and B, overlaps and fail-closed defaults (NP-E7, NP-E8) | N1-02 | `Rust` CI on the first N1 pull request (passing results count once it is merged) |
| T2-POL-02 | T2 | Hostile or invalid policy text rejected | N1-02 | `Rust` CI on the first N1 pull request (passing results count once it is merged) |
| T2-KS-01 | T2 | Protected IPv4 and IPv6 dropped with no VPN path (NP-E1, NP-E3) | N1-03 | NOT RUN |
| T2-KS-02 | T2 | Approved LAN exceptions and underlay exceptions still work with the path down (NP-U1…U4) | N1-03 | NOT RUN |
| T2-KS-03 | T2 | Rules survive a service crash and are repaired on restart (NP-E5) | N1-03, M5 | NOT RUN |
| T2-KS-04 | T2 | Apply and roll back leave no rules behind (NP-E6) | N1-03 | NOT RUN |
| T2-KS-05 | T2 | Network change, sleep/wake and reconnect leave no leak window (NP-E2, NP-E4) | M5, M7 | NOT RUN |
| T2-DNS-01 | T2 | Full tunnel: DNS only through the tunnel; ports 53/853 elsewhere dropped (NP-D1, NP-D5) | N1-03, M5 | NOT RUN |
| T2-DNS-02 | T2 | Split: home domains never sent to underlay DNS (NP-D2) | M5 | NOT RUN |
| T2-V6-01 | T2 | IPv6 dropped when the tunnel lacks IPv6; router advertisements cannot bypass (NP-6.1, NP-6.2) | N1-03, M5 | NOT RUN |
| T2-OV-01 | T2 | Overlapping subnets: home traffic never delivered locally (NP-S5) | M5 | NOT RUN |
| T2-GW-01 | T2 | Gateway drops destinations a device is not authorized for (NP-G1) | M5 | NOT RUN |
| T2-GW-02 | T2 | Device-to-device blocked by default; spoofed tunnel sources dropped (NP-G2, NP-G3) | M5 | NOT RUN |
| T2-J-A | T2 | Journey A end to end (NP-S2) | M5, G2 | NOT RUN |
| T2-J-B | T2 | Journey B end to end (NP-S1) | M5, G2 | NOT RUN |
| T2-J-AB | T2 | Journeys A and B through the home server (NP-S3) and the approved-NAS case (NP-S4) | M5, G2 | NOT RUN |
| T3-EN-01 | T3 | Expired, reused, wrong-fingerprint and over-attempted invitations rejected; no private key in any invitation (EN-1…6) | M6-01 | NOT RUN |
| T3-EN-02 | T3 | Administrator confirmation code flow (EN-7) | M6-01 | NOT RUN |
| T3-RV-01 | T3 | Revocation: new handshakes refused, sessions closed at once, persisted (RV-1, RV-2) | M6-01 | NOT RUN |
| T3-RV-02 | T3 | Changed server identity refused with a warning (RV-5) | M6-01 | NOT RUN |
| T3-KS-01 | T3 | Key files and stores have the stated protection; keys excluded from backups (KS-1…3) | M6, M7 | NOT RUN |
| T3-AD-01 | T3 | Node service rejects unauthorized local callers (AD-4) | M6 | NOT RUN |
| T3-ST-01 | T3 | Four independent status stages; "adapter up" never shown as connected (D-020) | N1-04 (model), M6-03 (UI) | NOT RUN |
| T3-LOG-01 | T3 | Logs and diagnostics contain no secrets, payloads or DNS names (PR-2…5) | M6-03 | NOT RUN |
| T3-OFF-01 | T3 | Offline self-hosted install and enrollment; no external network calls (LO-1, LO-2) | M6 | NOT RUN |
| T3-UI-01 | T3 | Window and tray behaviour: minimize keeps the connection; X (Exit and Disconnect) disconnects then exits; X (Send to System Tray) keeps it; GUI crash changes nothing (D-041) | N4-01/N4-06 (Linux), M7 | NOT RUN |
| T3-UI-02 | T3 | Both approved interface modes match their reference images; switching modes never touches the connection (D-037) | N4-01, N4-02, N4-08, M7 | NOT RUN |
| T3-UI-03 | T3 | Kill Switch and DNS Protection tiles show only verified states; Local Network Access tile shows the service's value (D-038–D-040) | N4-02, N4-04 | NOT RUN |
| T2-LD-01 | T2 | Lockdown Mode: blocking after disconnect, app exit, service restart and from boot; only NP-K2 exceptions (D-038) | N4-04 (lab), M5 | NOT RUN |
| T2-LAN-01 | T2 | Local Network Access: only directly attached subnets within caps; hostile DHCP/RA prefixes and overlaps blocked (D-040, NP-L1–L7) | N4-04 (lab), M5 | NOT RUN |
| T2-SW-01 | T2 | Profile switch while connected leaves no gap (D-041, NP-E2) | N4-03 (lab), M5 | NOT RUN |
| T2-DNS-02 | T2 | No DNS outside the permitted policy in any DNS mode or fallback; encrypted DNS validates certificates (D-039, NP-D7–D10) | N4-06 (settings), later DoH/DoT work order | NOT RUN |
| T3-PKG-01 | T3 | Signed install, update and uninstall with complete cleanup | M6-02, M7 | NOT RUN |
| T3-REL-01 | T3 | Signed builds, provenance and SBOM | M8-03 | NOT RUN |
| T4-PERF-01 | T4 | Reproducible throughput, latency and resource benchmarks | M4-04, M9-05 | NOT RUN |
| T5-TEN-01 | T5 | Tenant isolation with overlapping address space | M9 | NOT RUN |
| T5-CP-01 | T5 | Control-plane outage and bounded revocation propagation | M9 | NOT RUN |
| T5-DOS-01 | T5 | Fairness and bounded pre-authentication cost under abusive tenants | M9 | NOT RUN |

## Additional product requirements

| Feature | Source decision | Components | Responsible tasks | Test IDs / evidence |
| --- | --- | --- | --- | --- |
| Four official names | D-001 | all | M0-01 and every UI/docs task | T0: README/docs/UX name checks |
| Independent Layer 3 Rust core | D-003..D-005 | C-SVP, C-PKT | M2–M4 | Wire spec; Rust source/dependency inventory; lab interop |
| AES-256-GCM preferred, not pre-approved | D-010 | C-PKT | M2-01..05, M3-03 | Reviewed suite + hardware benchmarks (T4-PERF-01) |
| Always-untrusted physical network | D-006 | C-OSNET, C-POL | M1-02, N1-02, N1-03, M5, M7 | T2-POL-01, T2-KS-01…05 |
| User opt-in direct LAN while VPN exits elsewhere | D-006 | C-POL, C-OSNET | M1-02, N1-02, M5-01..03 | T2-POL-01, T2-KS-02, T2-J-AB |
| Version 1 journeys A and B | D-027 | C-POL, C-OSNET, C-GW | M1-02, N1-02, M5, G2 | T2-J-A, T2-J-B, T2-J-AB |
| Standalone, no mandatory vendor account | D-007 | C-ID, C-NODE | M1-03, M6-01..04 | T3-OFF-01 |
| No mandatory telemetry; minimal diagnostics | D-017 | C-APP, C-NODE | M1-03, M6-03 | T3-LOG-01, T3-OFF-01 |
| Easy end-user installation, no builds | D-008 | C-REL | M6-02..04, M7 | T3-PKG-01; novice usability tests |
| Enterprise/provider capability | D-009 | C-CP, C-GW | M9 | T5-TEN-01, T5-CP-01; real idle/active concurrency benchmarks |
| Measured performance goals | D-018 | C-PKT | M4-04, M9-05 | T4-PERF-01; no unmeasured claims |
| All Septuvo-authored VPN and Server code in Rust; external native components reviewed and inventoried | D-005, D-015, D-036 | all | M1-04, N4-01, M3-01, M7-01/04 | [spec/rust-inventory.md](spec/rust-inventory.md); [rust-gui-evaluation.md](reviews/rust-gui-evaluation.md); per-tree dependency audit |
| Platform order and honest support claims | D-014 | all | M1-04, M7 | [spec/platform-feasibility.md](spec/platform-feasibility.md); T2/T3 per OS |
| Desktop interfaces: Simple and Advanced, shared on Windows, Linux, macOS | D-037 | C-APP | N4 (Linux), M7 | T3-UI-02; [ui/desktop](ui/desktop/DEFAULT-UI.md) |
| Mandatory kill switch; Lockdown Mode | D-038 | C-APP, C-NODE, C-OSNET | N4-04, M5 | T2-LD-01, T3-UI-03 |
| Mandatory DNS leak prevention; configurable secure DNS | D-039 | C-APP, C-NODE | N4-06; DoH/DoT after the TLS decision | T2-DNS-02, T3-UI-03 |
| Local Network Access control | D-040 | C-APP, C-NODE, C-OSNET | N4-04, M5 | T2-LAN-01, T3-UI-03 |
| Window, tray and profile switching | D-041 (supersedes D-019) | C-APP, C-NODE | N4-01, N4-03, N4-06, M7 | T3-UI-01, T2-SW-01 |
| Update checking; signed installation later | D-042 | C-APP, C-REL | After the TLS decision; G3 | T3-PKG-01 |
| Four-stage connection status | D-020 | C-APP, C-NODE | N1-04, M6-03 | T3-ST-01 |
| Non-crypto Rust foundation (track N1) | D-029, D-030 | C-POL, C-OSNET, C-APP | N1-01…N1-04 | T0-SC-01, T2-POL-01/02, T2-KS-01…04, T3-ST-01 |
| Domain/trademark suitability | D-016 | — | Owner + counsel, outside protocol tasks | Formal clearance prior to commercialization |

## Blockers versus optional work

- **Hard stop:** no G1 means no implementation of SVP handshake, AEAD packet framing, security state machine or live tunnel.
- **Hard stop:** no G3 means no production security claim or public production release.
- **Hard stop:** no actual physical Android/iOS testing means no mobile platform support claim.
- **Hard stop:** no G4 means no “million users” or provider capacity claims.
- **Optional future feature:** Layer 2 bridging is explicitly *not* part of SVP v1.
- **Optional infrastructure:** coordination service or relay, never a hard dependency for supported standalone operation.
- **Unresolved platform facts:** under D-005 (2026-10-18) all Septuvo-authored VPN and Server code is Rust; external components may be in any language after review. Android (`VpnService` class) and Apple (Network Extension principal class) need a Septuvo class the OS loads by name; Windows needs an existing driver (D-036, options in [reviews/windows-tunnel-evaluation.md](reviews/windows-tunnel-evaluation.md)). Recorded in [spec/rust-inventory.md](spec/rust-inventory.md), to be resolved before each platform's work order.

## Evidence review rubric

For each row, reviewers should locate: (1) exact owner-accepted requirement, (2) linked approved design, (3) named task commit/PR, (4) a runnable test and raw results, (5) negative tests and exception handling, (6) explicit owner gate approval if required. If any is absent, mark **OPEN** or **NOT RUN** rather than assuming acceptance.

See [TASK_CATALOG.md](TASK_CATALOG.md), [TEST_MATRIX.md](TEST_MATRIX.md), and [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md).
