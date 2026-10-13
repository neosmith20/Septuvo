# Septuvo — decision register

All security-sensitive choices require explicit owner approval and, when identified by the security model, qualified external review. “Direction agreed” is a product requirement, **not** authorization to implement an original secure protocol.

| ID | Area | Decision / candidate | State | Next action |
| --- | --- | --- | --- | --- |
| D-001 | Brand | Septuvo = company/brand; Septuvo VPN = applications; Septuvo Protocol (SVP) = protocol; Septuvo Server = gateway product | Owner confirmed | Use consistently |
| D-002 | Tagline | “Security Without Compromise.” | Owner confirmed (working copy) | Avoid interpreting as a proven security claim |
| D-003 | Primary network layer | IPv4/IPv6 Layer 3; no core Layer 2 bridge | Direction agreed | Formalize routing and non-goals |
| D-004 | Protocol | Original, independent openly documented UDP VPN protocol, not a WireGuard fork | Direction agreed | Written specification + review before crypto code |
| D-005 | Language | Rust for all Septuvo-owned security-critical implementation, services and client logic | Direction agreed | Evaluate platform bindings; disclose exceptions |
| D-006 | Security posture | Every attached network untrusted; protected traffic fail closed; LAN allowed only by explicit destination/policy | Direction agreed | Define platform-specific enforcement and leak tests |
| D-007 | Self-hosting | Standalone private installation must work without a mandatory vendor account or cloud service | Direction agreed | Define enrollment and relay opt-in |
| D-008 | Usability | End users must install signed packages; guided connect/host/org roles; no component compilation | Direction agreed | Validate platform-specific feasibility |
| D-009 | Scale | One protocol/engine for home, business and multi-gateway providers | Goal | Load/capacity/tenant-isolation design and tests |
| D-010 | Data cipher | AES-256-GCM preferred; ChaCha20-Poly1305 possible suite | Candidate — **not approved protocol selection** | Threat/performance/dependency review |
| D-011 | Key establishment | X25519 + ML-KEM-768 hybrid | Candidate — **not approved handshake** | Specialist review of full construction |
| D-012 | Authentication | ML-DSA-65 + enrollment/trust anchors | Candidate — **not approved identity design** | Choose ownership, rotation, PKI and recovery semantics |
| D-013 | HKDF / handshake / AEAD nonces / packet framing | Details undecided | **BLOCKED** | G1 reviewed specification and owner approval |
| D-014 | OS/client priority | Linux lab first; later Windows/macOS/Android/iOS product integrations | Proposed sequencing | Owner approval in M0 |
| D-015 | Third-party dependencies | Audited, maintained and reviewed; no hidden C/C++ cryptographic engine | Policy direction | Dependency-review record before adding |
| D-016 | Availability / domain / marks | Septuvo name selected for project, not legally cleared | OPEN | Trademark/domain counsel and checks before commercialization |
| D-017 | Privacy/telemetry | No mandatory telemetry or SaaS; collect minimal diagnostics | Direction agreed | Define data-retention defaults, explicit opt-in |
| D-018 | Public performance goals | Competitive throughput, low latency and good battery life | Unmeasured aspiration | Controlled reproducible benchmarks |
| D-019 | Exit behavior | Client UI may close/minimize without dropping tunnel; intentional disconnect distinct | Product requirement | Test after desktop clients exist |
| D-020 | Connected status | Distinguish adapter active, authenticated peer, data reachability and DNS | Product requirement | Independent diagnostic probes and UX tests |

## Open questions requiring written decisions before relevant tasks

1. Identity trust model: locally pinned peer public keys, server enrollment, optional organization PKI, renewal and revocation.
2. Hybrid KEM combiner and transcript authentication; formal downgrade resistance and identity binding.
3. Anti-replay counters, cipher/epoch and unique AEAD nonce assignment across crashes, restarts and multiprocess workers.
4. Session resumption, packet/version negotiation, roaming authorization, reflection/replay resistance and DoS budgets.
5. Route conflict policy (home remote subnet overlapping current LAN), split rules, IPv6/DNS leak controls, platform kill-switch behavior.
6. Credential recovery, key storage on each OS, client re-enrollment after lost devices, revocation staleness during outages.
7. Control-plane architecture (standalone storage vs clustered HA), multi-tenancy boundaries and revocation deadlines.
8. OS API limitations: true Rust-only feasibility, mobile background behavior, virtual interfaces and signed installers.

Record any new decision as a new D-ID with rationale, rejected alternatives, consequences and owner authorization. **Never silently convert a candidate into a fact.**
