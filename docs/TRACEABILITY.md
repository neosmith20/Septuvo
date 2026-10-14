# Septuvo — requirement and evidence traceability

This is a **planned** traceability map. No listed test is considered run or passed. Test case IDs are proposed identifiers; test code/files must only be added under authorized work orders.

## Security requirements (S-01 through S-13)

| ID | Primary engineering owner/task | Planned evidence / negative test | Minimum gate |
| --- | --- | --- | --- |
| S-01 authenticated key agreement | M2-02/03, M3-03/04 | T1: wrong peer, forged auth, transcript mix-up, handshake order, version and role confusion | G1 design; G3 production |
| S-02 hybrid PQ goal | M2-01/02/05, M3-03 | T1: reviewed KEM/composition, future-recording threat, downgraded/cross-suite handshake rejected | G1 reviewed design; G3 audit |
| S-03 key lifecycle | M2-03, M3-03/04, M4-03 | T1: key derivation and separation, expiration, rekey with loss, recovery, stale key rejection | G1; G3 |
| S-04 AEAD nonce/replay | M2-03, M3-04, M4-03 | T1: crash/restart, worker concurrency, epoch overlap, counter rollover, duplicate/invalid ciphertext | G1; G3 |
| S-05 downgrade prevention | M2-02, M3-04 | T1: version negotiation tampering, cipher mismatch, PQ-required policy against old peers | G1; G3 |
| S-06 session replay/migration | M2-03, M3-04, M4-03, M7-05 | T1/T3: replay windows, rebind to new address, spoofed migration, Wi-Fi/mobile roam | G1; G3 |
| S-07 resource/DoS bounds | M2-03, M3-02, M5-04, M9-04 | T1/T5: malformed/oversized frames, pre-auth amplification and allocation, brute-rate fairness | G1; G3/G4 |
| S-08 hostile network fail closed | M1-02, M5-01/02, M7-01/05 | T2/T3: UDP endpoint failure, DNS/IPv6 leakage, interface roam, sleep, crash, LAN opt-in | G2 lab; G3 per OS |
| S-09 least-privilege access | M1-02, M4-02, M5-01/03, M9-04 | T2/T5: route vs authorization, forbidden NAS subnet, cross-tenant reachability | G2; G4 provider |
| S-10 safe enrollment/revocation | M1-03, M6-01 | T3: expired/replayed invite, private key exclusion, lost-device revoke and replacement | G3 |
| S-11 control-plane separation | M1-03, M9-01/02/03 | T5: role authz, stolen control token, limited stale policy, outage/isolation | G4 |
| S-12 signing/dependency/update | M3-01/03, M6-02, M7-03, M8-03 | T3/T5: dependency inventory, verified packages, rollback, provenance, secret scanning | G3 |
| S-13 privacy/diagnostics | M1-03, M6-03, M7-02 | T3: logs omit keys/data, consent/retention documented, “adapter up” ≠ authenticated tunnel | G3 |

## Additional product requirements

| Feature | Source decision | Responsible tasks | Evidence |
| --- | --- | --- | --- |
| Four official names | D-001 | M0-01 and every UI/docs task | T0: README/docs/UX name checks |
| Independent Layer 3 Rust core | D-003..D-005 | M2–M4 | Wire spec; Rust source/dependency inventory; lab interop |
| AES-256-GCM preferred, not pre-approved | D-010 | M2-01..05, M3-03 | Reviewed suite + hardware benchmarks |
| Always-untrusted physical network | D-006 | M1-02, M5, M7 | Hostile local-network and leak tests |
| User opt-in direct LAN while VPN exits elsewhere | D-006 | M1-02, M5-01..03 | Home NAS vs exit routing and firewall tests |
| Standalone no mandatory vendor account | D-007 | M6-01..04 | Fresh offline self-hosted installation |
| Easy end-user installation, no builds | D-008 | M6-02..04, M7 | Installer and novice usability tests |
| Enterprise/provider capability | D-009 | M9 | Separate real idle/active concurrency benchmarks and isolation |
| Measured performance goals | D-018 | M4-04, M9-05 | T4: reproducible lab benchmarks; no unmeasured claims |
| Third-party OS/native dependencies transparently inventoried | D-005, D-015 | M3-01, M7-01/04 | Platform matrix and dependency audit |
| Desktop UI status and close behavior | D-019, D-020 | M6-03, M7-02/03 | Connection-stage diagnostics and UX tests |
| Domain/trademark suitability | D-016 | Owner + counsel, outside protocol tasks | Formal clearance prior to commercialization |

## Blockers versus optional work

- **Hard stop:** no G1 means no implementation of SVP handshake, AEAD packet framing, security state machine or live tunnel.
- **Hard stop:** no G3 means no production security claim or public production release.
- **Hard stop:** no actual physical Android/iOS testing means no mobile platform support claim.
- **Hard stop:** no G4 means no “million users” or provider capacity claims.
- **Optional future feature:** Layer 2 bridging is explicitly *not* part of SVP v1.
- **Optional infrastructure:** coordination service or relay, never a hard dependency for supported standalone operation.
- **Unresolved platform fact:** literal 100% Rust end-to-end depends on OS-native VPN frameworks; Septuvo-owned security-critical code is Rust, all native glue must be disclosed.

## Evidence review rubric

For each row, reviewers should locate: (1) exact owner-accepted requirement, (2) linked approved design, (3) named task commit/PR, (4) a runnable test and raw results, (5) negative tests and exception handling, (6) explicit owner gate approval if required. If any is absent, mark **OPEN** or **NOT RUN** rather than assuming acceptance.

See [TASK_CATALOG.md](TASK_CATALOG.md), [TEST_MATRIX.md](TEST_MATRIX.md), and [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md).
