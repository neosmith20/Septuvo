# SVP — candidate constructions and Rust library analysis (M2-01)

**Status:** draft for owner review and, later, the G1 outside review. **This document chooses nothing.**
- It compares published, independently analysed constructions SVP could be assembled from (work order M2, rule 1) and recommends one direction for drafting M2-02.
- D-010, D-011, D-012 and D-024 remain **candidates**, and D-013 remains **blocked** until G1.
- Facts about standards and libraries were checked on **2026-10-15**; sources are listed at the end.

**License:** Apache-2.0 (open SVP specification, D-033).

## 1. What the construction must deliver

Drawn from the M1 requirements ([threats](../spec/threats.md), [network policies](../spec/network-policies.md), [identity and privacy](../spec/identity-and-privacy.md)) and D-032:

| # | Requirement | Source |
| --- | --- | --- |
| R1 | Mutual authentication of device and server, using identities pinned at enrollment; no certificate authority in version 1 | S-01, S-10, identity-and-privacy §2 |
| R2 | Confidentiality that survives a future quantum computer ("harvest now, decrypt later") and also a break of the post-quantum part (hybrid) | S-02, D-011, ADV-04 |
| R3 | Forward secrecy, and resistance to key-compromise impersonation and unknown-key-share attacks | S-01, S-03 |
| R4 | Authenticated negotiation; no downgrade | S-05 |
| R5 | Small, bounded work for unauthenticated packets; anti-amplification; cookie/retry mechanism | S-07, Q12 |
| R6 | Few round trips; works over UDP; message sizes compatible with real networks (IPv6 minimum MTU 1280 bytes) | D-004, D-018 |
| R7 | Device identity not readable by a passive observer (desirable) | S-13, AST-06 |
| R8 | **Existing published security analysis** of the construction, so the G1 reviewer checks our adaptation, not a new design | AGENTS.md (no invented cryptography) |
| R9 | Implementable in Rust with reviewable, licence-compatible libraries | D-005, D-015, D-033 |

## 2. Building blocks and their status

| Building block | Status (checked 2026-10-15) | Notes |
| --- | --- | --- |
| **ML-KEM** (FIPS 203) | Final NIST standard | ML-KEM-768: encapsulation key 1184 B, ciphertext 1088 B, shared secret 32 B. Plain ML-KEM does not give the strong "binding" properties some protocols assume (noted in the X-Wing and CFRG hybrid-KEM drafts) |
| **ML-DSA** (FIPS 204) | Final NIST standard | ML-DSA-65: public key 1952 B, signature 3309 B. Large for a UDP handshake |
| **X25519** (RFC 7748) | Standard, widely analysed | 32-byte keys |
| **X-Wing** hybrid KEM (ML-KEM-768 + X25519, SHA3-256 combiner) | IETF Internet-Draft `draft-connolly-cfrg-xwing-kem-11` (23 Sep 2026), Informational, **not yet an RFC** | Security argument in ePrint 2024/039. Binding-property proof still marked TODO. Sizes: encapsulation key 1216 B, ciphertext 1120 B |
| **Generic hybrid KEM combiners** | CFRG research-group draft `draft-irtf-cfrg-hybrid-kems-11` (7 May 2026), Informational | Defines UG/UK/CG/CK frameworks; binding analyses are "informal sketches" with proofs deferred |
| **Hybrid key exchange in TLS 1.3** (X25519MLKEM768 and others) | **RFC 10024** (August 2026) | TLS-specific; not by itself evidence for an SVP construction |
| **Noise Protocol Framework** (revision 34) | Widely deployed (WireGuard, others), extensive analysis | Classical (Diffie-Hellman) only |
| **PQNoise** (Angel, Dowling, Hülsing, Schwabe, Weber; ACM CCS 2022) | Peer-reviewed | KEM-based Noise patterns with generic proofs (fACCE model) |
| **KEMTLS** (Schwabe, Stebila, Wiggers; ACM CCS 2020) | Peer-reviewed | Authentication with KEMs instead of signatures |
| **Hybrid-WireGuard** (Lafourcade et al.; USENIX Security 2025) | Peer-reviewed; formal analysis with SAPIC+ (ProVerif, Tamarin, DeepSec) | 2-message hybrid handshake; fixes weaknesses found in the original PQ-WireGuard design |
| **HKDF** (RFC 5869) | Standard | Candidate D-024 |
| **AES-256-GCM** (NIST SP 800-38D), **ChaCha20-Poly1305** (RFC 8439) | Standards | Candidates D-010 |

## 3. Candidate approaches

### Approach A — Hybrid Noise-style handshake, authentication by static keys (recommended)

**What it is:** a two-message handshake in the style of WireGuard/Noise IK.
- The initiator already knows the responder's static public keys, which fits Septuvo: the device pins the server identity at enrollment, and the server stores device keys.
- Each side authenticates by its **static keys**: a classical X25519 key plus a post-quantum KEM key. No signatures are used.
- Ephemeral X25519 and ephemeral ML-KEM give forward secrecy, and both results feed one chained key schedule.

The closest analysed design is **Hybrid-WireGuard**. Its paper:
- found and fixed weaknesses in the earlier PQ-WireGuard design: an unknown-key-share issue that a model had wrongly passed, a reflection attack under compromised randomness, missing identity hiding, non-standard KEM use, and a false denial-of-service assumption;
- proves the hybrid version secure if *either* the classical or the post-quantum part holds;
- shows identity hiding under conditions.

PQNoise gives generic proofs for KEM-based Noise patterns.

| | |
| --- | --- |
| **Strengths** | Published formal analysis of nearly this exact shape (R8). 1 round trip (R6). No large signatures. Fits pinned identities (R1). Hybrid by construction (R2). Identity hiding (R7). Cookie/anti-DoS design inherited from WireGuard-style protocols (R5) |
| **Weaknesses** | Static post-quantum keys must be exchanged in advance: fine for Septuvo's enrollment, but enrollment must carry them. Message size depends on the KEM: with ML-KEM-768 everywhere the handshake exceeds the 1280-byte IPv6 minimum (the paper measured 1540/1384 B with ML-KEM level 3), so either fragmentation or a different static KEM is needed (owner question **M2-Q3**). The analysed design uses specific choices (ML-KEM level 1 ephemeral, Classic McEliece static); any change from those is a **deviation the G1 reviewer must check** |
| **Rust** | The paper's Rust artifact builds on `wireguard-rs` and uses liboqs (research quality). `clatter` implements PQNoise and hybrid Noise in pure Rust but is unaudited, from a single maintainer. Usable as references, not as dependencies without review |

### Approach B — Signature-authenticated key exchange with a hybrid KEM (TLS-1.3-like)

**What it is:**
- A SIGMA/TLS-1.3-style exchange.
- Ephemeral **X-Wing** (or another CFRG hybrid KEM) for key agreement.
- **ML-DSA-65 + Ed25519** signatures over the transcript for authentication (matches candidate D-012).

| | |
| --- | --- |
| **Strengths** | Clean separation: a standard hybrid KEM used as a black box; signatures match D-012; the key-schedule style follows TLS 1.3, which has extensive analysis |
| **Weaknesses** | **No published analysis of this exact composition.** We would be composing pieces, so many elements become BLOCKED for the reviewer (R8). Large messages: an ML-DSA-65 signature plus public key is about 5.2 KB, so each handshake needs several packets and robust fragmentation (R6, R5). X-Wing is still a draft, and its binding proof is incomplete |
| **Rust** | ML-DSA: RustCrypto `ml-dsa` and others, **none independently audited** (section 5) |

### Approach C — Reuse TLS 1.3 (or QUIC) for the handshake

**What it is:**
- A standard TLS 1.3 / DTLS 1.3 / QUIC handshake with RFC 10024 hybrid key exchange.
- Raw public keys instead of certificates.
- An SVP data channel keyed from the TLS exporter.

The packet-carrying part could go further and use QUIC with the MASQUE CONNECT-IP standard.

| | |
| --- | --- |
| **Strengths** | The most-analysed key exchange in existence; mature Rust libraries for TLS (`rustls`, which supports X25519MLKEM768 through its aws-lc-rs provider) |
| **Weaknesses** | Post-quantum *authentication* in TLS (ML-DSA certificates and keys) is not yet settled. DTLS 1.3 support in Rust is immature. TLS brings a large surface (certificates, extensions) that Septuvo's pinned-key model does not need. **Conflicts with D-004** (an original, independent SVP protocol) if taken whole |

### Considered and not recommended

**Rosenpass-style layering:** a separate post-quantum exchange (Rosenpass implements PQ-WireGuard) feeds a pre-shared key into an unchanged classical WireGuard tunnel, refreshed about every two minutes.
- It is deployed (for example in NetBird).
- But the 2025 analysis notes it amounts to a 4-message hybrid handshake with higher latency, and several of its security claims and countermeasures lack symbolic proof.
- It also depends on WireGuard itself, which D-004 rules out as the core.

## 4. Comparison

| Criterion | A: Hybrid Noise-style | B: Signatures + hybrid KEM | C: TLS 1.3 / QUIC |
| --- | --- | --- | --- |
| Published analysis of the composition (R8) | **Yes**, close match (Hybrid-WireGuard 2025, PQNoise 2022) | Partial (components only) | Yes for TLS; not for PQ authentication or the VPN layer |
| Hybrid confidentiality (R2) | Yes | Yes | Yes (RFC 10024) |
| Post-quantum authentication | Yes (static KEM keys) | Yes (ML-DSA) | Not settled |
| Round trips (R6) | 1 | 1–1.5, more packets | 1–2 |
| Handshake size | ~1.1–1.5 KB per message (KEM-dependent) | ~5–7 KB per side | Several KB |
| Fits pinned enrollment (R1) | Natural | Good | Needs raw-public-key mode |
| Identity hiding (R7) | Shown (conditional) | Possible | Possible (encrypted handshake) |
| Expected number of BLOCKED novel elements | **Lowest** | High | Medium (VPN layer) |
| Consistent with D-004 | Yes, with owner confirmation (M2-Q1) | Yes | No, if taken whole |

## 5. Rust library maturity (implementation is after G1; this informs the choice)

| Library | Algorithms | Language / licence | Assurance (as found 2026-10-15) |
| --- | --- | --- | --- |
| RustCrypto `ml-kem` 0.3.2 (May 2026) | ML-KEM | Pure Rust; Apache-2.0 or MIT | Tested against NIST vectors; README: **"has never been independently audited"** |
| RustCrypto `ml-dsa` | ML-DSA | Pure Rust; Apache-2.0 or MIT | Same: **not audited** |
| `libcrux` (Cryspen) | ML-KEM, ML-DSA, others | Rust with formal verification (hax/F*) | Formal verification claims; in **February 2026 an independent analysis reported specification and proof bugs** in the verified ML-KEM code and verification run in lax mode, disputed by the vendor. **Treat the verification claims with caution until independently re-checked** |
| `aws-lc-rs` | ML-KEM, ML-DSA (unstable flag), X25519, AES-GCM | Rust bindings to **AWS-LC (C)** | FIPS work for the library; **post-quantum code not independently audited**. Contains a C cryptographic engine, so using it needs an explicit **D-015 exception** |
| `liboqs` / `oqs` | Many | C library | Its README says it is **not recommended for production** |
| RustCrypto `aes-gcm`, `chacha20poly1305` | AEADs | Pure Rust | **Reviewed by NCC Group (2020)**; re-check the versions reviewed |
| `x25519-dalek` | X25519 | Pure Rust | Widely used; audit history to be confirmed at dependency review |
| `snow` / `clatter` | Noise / PQNoise | Pure Rust | `clatter` 2.x: **not audited**, single maintainer |

**Summary:** an independent survey (Project Eleven, around 2025) found **no Rust post-quantum implementation with an independent audit**, and that still appeared true on 2026-10-15. This is a real project risk. It does not block the specification, which stays library-independent, but it needs a plan before M3 (owner question **M2-Q4**).

## 6. Recommendation

1. **Draft M2-02 on Approach A:** a two-message hybrid handshake that follows the **Hybrid-WireGuard** construction as closely as Septuvo's requirements allow, citing PQNoise for the general pattern. Every difference from the analysed design is listed and marked **BLOCKED — needs G1 review**: KEM choices, identity encoding, cookie mechanism, labels, roaming, versioning.
2. **Keep the cryptographic candidates as candidates.**
   - **ML-KEM-768** for the post-quantum part (D-011), with **X25519**; the static-key KEM is question M2-Q3.
   - **AES-256-GCM** or **ChaCha20-Poly1305** for data (D-010).
   - **HKDF** or the analysed design's own KDF (D-024). Following the analysed design keeps the reviewer's job smallest.
3. **Signatures (D-012) are not needed in the handshake under Approach A.** ML-DSA remains a candidate for signing enrollment artefacts and software updates.
4. **Library choice waits until after G1.** The specification must be implementable by at least two independent libraries.

## 7. Owner questions (M2)

| Q | Question | Recommendation |
| --- | --- | --- |
| M2-Q1 | May SVP's handshake **follow a published, formally analysed design** (Hybrid-WireGuard), with every deviation reviewed, while SVP stays an independent protocol (D-004)? | **Yes.** Following analysed work is the safest path. "Independent" means our own specification and implementation, not new cryptography |
| M2-Q2 | Authenticate devices and servers with **static keys (X25519 + post-quantum KEM)** instead of ML-DSA signatures in the handshake? | **Yes.** Much smaller handshake, analysed design; ML-DSA kept for signing updates and enrollment data |
| M2-Q3 | Static post-quantum KEM: **ML-KEM-768** (one algorithm family, handshake messages above 1280 B, so fragmentation is needed) or **Classic McEliece** (small messages, but public keys of hundreds of kilobytes, carried at enrollment)? | **ML-KEM-768**, for simplicity and consistency with D-011; M2-03 specifies fragmentation and its denial-of-service limits. Revisit only if the G1 reviewer advises otherwise |
| M2-Q4 | Dependency strategy before M3, given no audited Rust post-quantum library | Decide after G1. Prefer pure-Rust (D-015) and cross-test two independent implementations against NIST vectors. Look for audit funding (for example through the same programs as the G1 review) |

All four are category C (security architecture). Your answers steer the drafting; the final approval is the G1 review plus your `GATE G1 PASSED`.

## Sources

- FIPS 203 (ML-KEM): <https://csrc.nist.gov/pubs/fips/203/final>
- FIPS 204 (ML-DSA): <https://csrc.nist.gov/pubs/fips/204/final>
- X-Wing draft -11: <https://www.ietf.org/archive/id/draft-connolly-cfrg-xwing-kem-11.html>
- CFRG hybrid KEMs draft -11: <https://www.ietf.org/archive/id/draft-irtf-cfrg-hybrid-kems-11.html>
- RFC 10024, PQ/T hybrid key agreement for TLS 1.3: <https://www.rfc-editor.org/rfc/rfc10024>
- Post Quantum Noise (CCS 2022): <https://eprint.iacr.org/2022/539>
- KEMTLS (CCS 2020): <https://eprint.iacr.org/2020/534/>
- Lafourcade et al., "A Tale of Two Worlds, a Formal Story of WireGuard Hybridization" (USENIX Security 2025): <https://www.usenix.org/conference/usenixsecurity25/presentation/lafourcade>
- NetBird on integrating Rosenpass: <https://netbird.io/knowledge-hub/how-we-integrated-rosenpass>
- Project Eleven, "The state of post-quantum cryptography in Rust": <https://blog.projecteleven.com/posts/the-state-of-post-quantum-cryptography-in-rust-the-belt-is-vacant>
- Symbolic Software on libcrux verification (12 Feb 2026): <https://symbolic.software/blog/2026-02-12-cryspen-response>
- RustCrypto `ml-kem`: <https://docs.rs/crate/ml-kem/latest>
- NCC Group review of RustCrypto AES-GCM and ChaCha20-Poly1305: <https://www.nccgroup.com/research/public-report-rustcrypto-aesgcm-and-chacha20pluspoly1305-implementation-review/>
- `clatter` (PQNoise in Rust): <https://docs.rs/crate/clatter/2.1.0>
- rustls defaults (post-quantum key exchange): <https://doc.servo.org/rustls/manual/_05_defaults/index.html>
