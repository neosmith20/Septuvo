# Septuvo

**Security Without Compromise.**

*Private by design. Powerful by nature. Simple for everyone.*

> **Project status: design / pre-implementation.** There is no working Septuvo VPN, published SVP wire protocol, audited cryptographic implementation, or production release yet. Statements here describe intended capabilities, not demonstrated security or performance.

**Septuvo** is an independent Layer 3 VPN project intended to connect devices and private networks securely, simply, and at scale. It aims to serve people accessing a home media server as well as organizations operating many gateways—without requiring ordinary users to compile components or manage cryptographic settings.

## Official product names

| Name | Role |
| --- | --- |
| **Septuvo** | Company and primary brand |
| **Septuvo VPN** | Consumer and business VPN software |
| **Septuvo Protocol (SVP)** | Independent networking protocol, to be openly specified |
| **Septuvo Server** | Self-hosted and commercial gateway software |

The project intends to develop an original VPN wire protocol and a Rust-native core, not a fork or rebranding of WireGuard, OpenVPN or Tailscale.

## Product goals

- **Untrusted by default:** assume every attached Wi-Fi, Ethernet and cellular network can be hostile. Offer explicitly approved local network access and advanced split routing without weakening unrelated tunnel traffic.
- **Post-quantum-conscious security:** evaluate hybrid classical/post-quantum key establishment, authenticated post-quantum-capable identities and audited symmetric encryption. The cryptographic construction is **not yet selected, proven, or audited**.
- **High performance:** efficient UDP packet transport, runtime hardware acceleration, stable roaming and low resource usage, measured with reproducible benchmarks.
- **Easy installation:** signed installer, guided client/server setup, secure enrollment, meaningful connection diagnostics and clear LAN/security preferences. No end-user compilation or manual crate selection.
- **Scalable architecture:** the same wire protocol and Rust packet engine for self-hosted personal connections, business gateways and commercial-provider clusters.
- **User ownership:** standalone self-hosting without a mandatory account, with optional coordination or relay services.

## Intended architecture (not an implemented specification)

| Area | Direction | Status |
| --- | --- | --- |
| Network model | Layer 3 IP tunnel, IPv4 + IPv6, UDP | Design intent |
| Codebase | First-party protocol, networking, security logic and services in Rust | Design intent; OS integration constraints to verify |
| Symmetric data encryption | AES-256-GCM preferred; ChaCha20-Poly1305 evaluated as an optional supported suite | Candidate |
| Key establishment | X25519 + ML-KEM-768 hybrid | Candidate; specialist review required |
| Identity/authentication | ML-DSA-65 and carefully designed enrollment/trust model | Candidate; specialist review required |
| Packet layout, handshake transcript, nonce construction, key rotation | **Undecided** | BLOCKED until reviewed and approved |
| Local LAN | Explicitly approved destinations, never implicit trust of RFC1918 ranges | Requirement |
| Public performance/security claims | Evidence and independent review required | Not achieved |

**AES-256-GCM and ML-KEM do different jobs.** AES encrypts data; ML-KEM helps establish secrets. Neither one alone guarantees a post-quantum secure VPN.

## Documentation and starting point

1. [AGENTS.md](AGENTS.md) — mandatory rules for every contributor.
2. [Agent operations](docs/AGENT_OPERATIONS.md) — dispatch, tasks and approvals.
3. [Task catalog](docs/TASK_CATALOG.md) — 43 fixed base task cards, plus authorized subtasks.
4. [Owner approvals and status](docs/STATUS.md) — gate/task ledger, dated permissions.
5. [Roadmap](docs/ROADMAP.md) — order and G0–G4 gates.
6. [Architecture](docs/ARCHITECTURE.md) and [security model](docs/SECURITY_MODEL.md) — technical requirements.
7. [Protocol review checklist](docs/PROTOCOL_REVIEW_CHECKLIST.md) and [test matrix](docs/TEST_MATRIX.md).
8. [Decision register](docs/DECISIONS.md), including D-021 (pre-G1 test vectors deferred until after G1).
9. [Canonical work order / handoff](docs/WORK_ORDER_TEMPLATE.md) and [traceability](docs/TRACEABILITY.md).
10. [Security policy](SECURITY.md) and [GitHub security setup](docs/security/GITHUB_SECURITY_SETUP.md).
11. [Brand kit](brand/README.md) and [brand guidelines](brand/Septuvo_Brand_Guidelines.pdf).

**Start with M0-01 documentation audit only after a scoped owner-approved work order.** All gates are pending; no agent is authorized to invent SVP cryptography or skip a review.

## Current availability

This repository contains planning documents, repository security checks, an approved visual **brand kit**, and the Apache-2.0 license, **not an installable VPN**. Brand/domain/trademark clearance has not been completed. There is no committed public release date.

## References

- [NIST FIPS 203 (ML-KEM)](https://csrc.nist.gov/pubs/fips/203/final)
- [NIST FIPS 204 (ML-DSA)](https://csrc.nist.gov/pubs/fips/204/final)
- [IETF RFC 10024 (hybrid TLS key exchange)](https://www.rfc-editor.org/info/rfc10024) — TLS-specific; not proof of an SVP construction.
- [IETF RFC 5869 (HKDF)](https://www.rfc-editor.org/rfc/rfc5869)
- [NIST SP 800-38D (GCM)](https://csrc.nist.gov/pubs/sp/800/38/d/final)

Licensed under [Apache-2.0](LICENSE).
