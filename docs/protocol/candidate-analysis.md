# SVP — candidate constructions and Rust library analysis (M2-01)

**Status:** draft, revised after owner review on 2026-10-15. **This document chooses nothing and approves nothing.**
- It compares published, independently analysed constructions SVP could be assembled from (work order M2, rule 1) and names a **provisional leading candidate** for drafting M2-02.
- D-010, D-011, D-012 and D-024 remain **candidates**, and D-013 remains **blocked**, until G1.
- Facts about standards and libraries were checked on **2026-10-15**; sources are at the end.

**License:** Apache-2.0 (open SVP specification, D-033).

> **Scope of any security statement here.** A published analysis covers *the analysed protocol exactly as specified, under its stated assumptions*. It does **not** prove Septuvo's adapted version secure. Every Septuvo change — parameters, key distribution, encodings, denial-of-service handling, versioning, roaming — needs its own review at G1.

## 1. What the construction must deliver

Drawn from the M1 requirements ([threats](../spec/threats.md), [network policies](../spec/network-policies.md), [identity and privacy](../spec/identity-and-privacy.md)) and D-032:

| # | Requirement | Source |
| --- | --- | --- |
| R1 | Mutual authentication of device and server, using identities pinned at enrollment; no certificate authority in version 1 | S-01, S-10, identity-and-privacy §2 |
| R2 | Confidentiality that survives a future quantum computer ("harvest now, decrypt later") and also a break of the post-quantum part (hybrid) | S-02, D-011, ADV-04 |
| R3 | Forward secrecy, and resistance to key-compromise impersonation (KCI) and unknown-key-share attacks | S-01, S-03 |
| R4 | Authenticated negotiation; no downgrade | S-05 |
| R5 | Small, bounded work and memory for unauthenticated packets; anti-amplification | S-07, Q12 |
| R6 | Few round trips over UDP; message sizes that work on real networks (IPv6 minimum MTU 1280 bytes) | D-004, D-018 |
| R7 | Device identity not readable by a passive observer (desirable) | S-13, AST-06 |
| R8 | **Existing published security analysis** of the construction, so the G1 reviewer checks our adaptation rather than a new design | AGENTS.md (no invented cryptography) |
| R9 | Implementable in Rust with reviewable, licence-compatible libraries | D-005, D-015, D-033 |

## 2. Building blocks and their status

| Building block | Status (checked 2026-10-15) | Sizes and notes |
| --- | --- | --- |
| **ML-KEM** (FIPS 203) | Final NIST standard | ML-KEM-512 (level 1): encapsulation key 800 B, ciphertext 768 B. ML-KEM-768 (level 3): encapsulation key 1184 B, ciphertext 1088 B. Plain ML-KEM does not give the strong "binding" properties some protocols assume (noted in the X-Wing and CFRG hybrid-KEM drafts) |
| **Classic McEliece** | **Not a NIST FIPS standard.** Submitted to ISO; described in IETF Internet-Drafts (`draft-josefsson-cfrg-mceliece-considerations-00`, June 2026) | mceliece348864 (level 1): public key 261,120 B, ciphertext 96 B. mceliece460896 (level 3): public key 524,160 B, ciphertext 156 B. Small ciphertexts but very large public keys |
| **ML-DSA** (FIPS 204) | Final NIST standard | ML-DSA-65: public key 1952 B, signature 3309 B |
| **X25519** (RFC 7748) | Standard, widely analysed | 32-byte keys |
| **X-Wing** hybrid KEM (ML-KEM-768 + X25519) | IETF Internet-Draft `draft-connolly-cfrg-xwing-kem-11` (23 Sep 2026), Informational, **not an RFC** | Security argument in ePrint 2024/039; binding proof still marked TODO. Encapsulation key 1216 B, ciphertext 1120 B |
| **Generic hybrid KEM combiners** | CFRG research-group draft `draft-irtf-cfrg-hybrid-kems-11` (7 May 2026), Informational | Binding analyses are "informal sketches", proofs deferred |
| **Hybrid key exchange in TLS 1.3** | **RFC 10024** (August 2026) | TLS-specific |
| **Noise Protocol Framework** (revision 34) | Widely deployed, extensively analysed | Classical (Diffie-Hellman) only |
| **PQNoise** (ACM CCS 2022) | Peer-reviewed; generic proofs in the fACCE model | KEM-based Noise patterns |
| **KEMTLS** (ACM CCS 2020) | Peer-reviewed | Authentication with KEMs instead of signatures |
| **Hybrid-WireGuard** (Lafourcade et al., USENIX Security 2025) | Peer-reviewed; symbolic analysis with SAPIC+ (ProVerif, Tamarin, DeepSec) | 2-message hybrid handshake. Uses **Classic McEliece level 3 for static keys and ML-KEM level 1 for ephemeral keys**; the paper names security levels, not parameter-set names |
| **HKDF** (RFC 5869), **AES-256-GCM** (SP 800-38D), **ChaCha20-Poly1305** (RFC 8439) | Standards | Candidates D-024, D-010 |

## 3. Candidate approaches

### Approach A — Hybrid Noise-style handshake, authenticated by static keys (provisional leading candidate)

**What it is:** a two-message handshake in the style of WireGuard/Noise IK.
- The initiator already knows the responder's static public keys. Each side authenticates with **static keys** (X25519 plus a post-quantum KEM key), with no handshake signatures.
- Ephemeral X25519 and ephemeral ML-KEM provide forward secrecy, and both feed one chained key schedule.

**Research basis, not an approved construction.** The closest analysed design is **Hybrid-WireGuard**. Its paper:
- found and fixed weaknesses in the earlier PQ-WireGuard design: an unknown-key-share issue a prior model had wrongly passed, a reflection attack under compromised randomness, missing identity hiding, non-standard KEM use, and a false denial-of-service assumption;
- reports, for its exact construction and assumptions, that the hybrid protocol is attacked only if *both* its classical and post-quantum parts are, together with key secrecy, forward secrecy and conditional identity hiding.

PQNoise gives generic proofs for KEM-based Noise patterns. **Neither result transfers automatically to Septuvo's adapted protocol** (see the note at the top).

| | |
| --- | --- |
| **Strengths** | Published formal analysis of a closely related protocol (R8). One round trip (R6). No large signatures. Fits pinned identities (R1). Hybrid by construction (R2). Identity hiding under conditions (R7) |
| **Weaknesses and open points** | Static post-quantum keys must be distributed in advance (§4). The analysed parameter choices (ML-KEM level 1 ephemeral, Classic McEliece level 3 static) differ from candidate D-011, and every substitution changes sizes (§5) and needs review. Some analysed properties assume a **secret pre-shared key (PSK)** that Septuvo does not yet define (§4). **Denial-of-service protection is not inherited** (§6) |
| **Rust** | The paper's Rust artifact builds on `wireguard-rs` with liboqs (research quality). `clatter` implements PQNoise/hybrid Noise but is unaudited (§7). References only, not dependencies |

### Approach B — Signature-authenticated key exchange with a hybrid KEM (TLS-1.3-like)

**What it is:** a SIGMA/TLS-1.3-style exchange using ephemeral **X-Wing** for key agreement and **ML-DSA-65 + Ed25519** signatures for authentication (candidate D-012).

- **Strengths:** clean use of a standard hybrid KEM as a black box; signatures match D-012; a TLS-1.3-like key schedule.
- **Weaknesses:**
  - **No published analysis of this exact composition**, so many elements would be BLOCKED (R8).
  - About 5.2 KB of signature plus public key per side, so several packets per message (R5, R6).
  - X-Wing is still a draft.

### Approach C — Reuse TLS 1.3 / QUIC for the handshake

**What it is:** standard TLS 1.3 / DTLS 1.3 / QUIC with RFC 10024 hybrid key exchange and raw public keys, then an SVP data channel keyed from the TLS exporter.

- **Strengths:** the most-analysed key exchange; mature Rust TLS (`rustls` offers X25519MLKEM768 through its aws-lc-rs provider).
- **Weaknesses:**
  - Post-quantum authentication in TLS is not settled.
  - DTLS 1.3 in Rust is immature.
  - It brings a large surface that Septuvo does not need.
  - Taken whole, it conflicts with D-004.

### Considered and not recommended

**Rosenpass-style layering:** a separate post-quantum exchange feeding a pre-shared key into an unchanged WireGuard tunnel, refreshed about every two minutes.
- The 2025 analysis describes it as effectively a 4-message hybrid handshake, with several claims and countermeasures lacking symbolic proof.
- It depends on WireGuard itself (D-004).

## 4. Authentication, identity binding, enrollment, revocation and KCI in Approach A (provisional, M2-Q2)

**How each property would work.** Every row is a provisional design direction for M2-02. **None is approved before G1.**

| Property | How it would work in Approach A | Status for G1 |
| --- | --- | --- |
| **Identities** | Each device and the server have a static X25519 key pair and a static post-quantum KEM key pair, generated on the machine (ID-1). The identity is the pair of public keys; the fingerprint is a hash of both | Encoding and fingerprint construction: **BLOCKED** |
| **Server authenticated to device** | The device encapsulates to the server's static KEM key and performs a static Diffie-Hellman with the server's static X25519 key. Only the holder of both private keys derives the session keys. The server's reply is encrypted and authenticated under keys that depend on them, which confirms the key to the device | Follows the analysed design; deviations **BLOCKED** |
| **Device authenticated to server** | The device's static identity is sent encrypted. The server looks it up among enrolled devices only, encapsulates to that device's static KEM key and uses its static X25519 key. **Key confirmation for the server is implicit** until the device's first valid encrypted packet, so the server must not report the peer as authenticated (D-020) or forward traffic before that packet arrives | Server-side state rules: **BLOCKED** |
| **Identity binding** | Both static public keys (classical and post-quantum) are bound into the key derivation, as in the paper's fix: the analysed design hashes both parties' post-quantum static keys into the key schedule and encrypts a hash of the initiator's classical and post-quantum keys. This prevents mixing one party's classical key with another's post-quantum key | Exact transcript and labels: **BLOCKED** |
| **Pre-shared key** | The analysed design uses a PSK as a MAC key and in key derivation; some properties (for example resistance when long-term keys leak) **rely on it being truly secret**. Septuvo has no per-device PSK yet. Options: create one during enrollment over the authenticated enrollment channel, or show the needed properties hold without one | **Open — a key G1 question** |
| **Enrollment** | The invitation carries the server's fingerprint (EN-1 to EN-9). The device fetches the server's full public keys over the enrollment connection, checks them against the fingerprint, and sends its own public keys. With Classic McEliece, about 0.5 MB of public key moves in each direction | Enrollment protocol: specified in M2-02, **BLOCKED** until reviewed |
| **Revocation** | The server removes the device's static keys from its enrolled set and closes the device's sessions (RV-1). Without the device's public keys, no new handshake can complete. A lost server key means re-enrolling devices (RV-5) | Follows M1 |
| **Key-compromise impersonation (KCI)** | The paper lists KCI resistance for its construction as following from its message-agreement properties, not as a separately modelled property. **Septuvo must re-verify KCI for its adapted protocol**, including the PSK choice above | **Open for G1** |

## 5. Packet sizes for Septuvo's substitutions (M2-Q3)

**How these numbers were derived.** The paper's Table 8 gives totals including IPv6 and UDP headers:
- Hybrid-WireGuard: InitHello 1156 B and RespHello 1064 B;
- with ML-KEM level 3: 1540 B and 1384 B.

The paper does not list every field size, so each row below starts from those totals and changes **only the key and ciphertext fields**, using the FIPS 203 and Classic McEliece sizes in §2:
- **Ephemeral ML-KEM:** the InitHello carries the encapsulation key, and the RespHello carries the ciphertext.
- **Static KEM:** each message carries one ciphertext.

Moving from level 1 to level 3 ephemeral reproduces the paper's own level-3 totals (+384 B and +320 B), which cross-checks the method. **These are estimates until M2-03 defines Septuvo's own wire format.**

| Ephemeral KEM | Static KEM | InitHello (IPv6 / IPv4) | RespHello (IPv6 / IPv4) | Fits 1280 (IPv6 minimum)? | Fits 1500 (typical IPv4)? |
| --- | --- | --- | --- | --- | --- |
| ML-KEM-512 | McEliece-348864 | 1096 / 1076 | 1004 / 984 | Yes | Yes |
| **ML-KEM-512** | **McEliece-460896** *(paper's configuration)* | **1156 / 1136** | **1064 / 1044** | **Yes** | Yes |
| ML-KEM-768 | McEliece-348864 | 1480 / 1460 | 1324 / 1304 | **No** | Yes |
| ML-KEM-768 | McEliece-460896 | 1540 / 1520 | 1384 / 1364 | **No** | **No** (InitHello) |
| ML-KEM-512 | ML-KEM-512 | 1768 / 1748 | 1676 / 1656 | No | No |
| ML-KEM-512 | ML-KEM-768 | 2088 / 2068 | 1996 / 1976 | No | No |
| ML-KEM-768 | ML-KEM-768 | 2472 / 2452 | 2316 / 2296 | No | No |

**Can fragmentation be avoided?**
- **Only with level-1 ephemeral ML-KEM (ML-KEM-512) and a Classic McEliece static key.** That is the paper's own configuration.
- **With ML-KEM-768 for the ephemeral exchange (candidate D-011), the InitHello cannot fit in 1280 bytes with any static KEM.** The ephemeral encapsulation key (1184 B) plus the ephemeral X25519 key and headers already take 1264 B before any other field.
- Using IPv4's typical 1500-byte path works only with a level-1 McEliece static key, and fails on IPv6-minimum links, many tunnels and mobile networks.

**Implications of each direction:**

| Choice | Security | Memory and enrollment | Denial of service |
| --- | --- | --- | --- |
| **Stay with the paper's parameters** (ML-KEM-512 ephemeral, McEliece-460896 static) | Matches the analysed configuration most closely. Ephemeral exchange at NIST level 1, below D-011's level 3. McEliece is conservative but **not a NIST standard** | Server stores about **0.5 MB per enrolled device** (100 devices ≈ 52 MB; 10,000 ≈ 5.2 GB). Key generation is expensive (measure before choosing). Enrollment moves about 0.5 MB each way | No fragmentation, so no reassembly state. McEliece decapsulation cost before authentication must be bounded (§6) |
| **ML-KEM-768 throughout** (D-011 as written) | Level 3 everywhere; one algorithm family; **deviates from the analysed parameters**, so the reviewer must confirm the substitution | 1184 B per device public key (10,000 devices ≈ 12 MB). Simple enrollment | **Fragmentation required**, 2 fragments per message. Reassembly buffers for *unauthenticated* data are a memory-exhaustion target; fragments can be forged or dropped to disrupt handshakes. Needs strict per-source and global reassembly budgets, a fragment-count limit, and a stateless cookie **before** reassembly state is kept |
| **Mixed** (ML-KEM-768 ephemeral, McEliece static) | Level-3 ephemeral; static keys as analysed | As the paper's configuration | Fragmentation still needed on IPv6-minimum paths (1540 B), so both costs apply |

**Not decided.** M2-Q3 stays open. M2-02 will be written so that the KEM parameter sets are **named parameters**, not fixed, and the G1 reviewer will be asked for a recommendation. M2-03 specifies application-level fragmentation only if the chosen parameters need it; IP-level fragmentation will not be relied on.

## 6. Denial of service is **not** inherited

WireGuard-style cookie and MAC mechanisms are **not** carried over as a proven property:
- The 2025 paper did **not** analyse denial-of-service resistance. It showed that the original PQ-WireGuard's protection rested on a **false assumption**: that static public keys stay secret. An attacker who knows them can force expensive work.
- In the analysed fix, the responder must decapsulate and decrypt **before** checking the first MAC, so post-quantum decapsulation happens before any authentication.

**Septuvo needs its own pre-authentication design, specified in M2-03 and reviewed at G1.** At minimum:
- a stateless return-routability (cookie) step used under load before any expensive decapsulation or state allocation;
- an **anti-amplification rule:** never send more bytes to an unverified address than received from it;
- per-source and global budgets for CPU, handshake state and, if used, fragment reassembly, with fixed caps and early rejection of malformed or oversized input;
- behaviour under load that degrades by refusing new handshakes, never existing sessions;
- targets measured in the G2 lab (S-07, Q12).

## 7. Rust library evidence, version by version (implementation is after G1)

The table records **what was checked, for which version, on 2026-10-15**. It does not claim anything about versions not listed or about audits not published.

| Library | Version checked | Assurance evidence found | Not verified |
| --- | --- | --- | --- |
| RustCrypto `ml-kem` | **0.3.2** (released 2026-05-10) | README: *"has never been independently audited! USE AT YOUR OWN RISK!"* Tested against NIST vectors. Apache-2.0 or MIT | — |
| RustCrypto `ml-dsa` | **0.1.1** (released 2026-06-05) | README: *"has never been independently audited! USE AT YOUR OWN RISK!"* Apache-2.0 or MIT | — |
| `libcrux` (ML-KEM) | Specific version **not checked** | Vendor claims formal verification (hax/F*). Independent posts by Symbolic Software (5 and 12 Feb 2026) report vulnerabilities and three bugs in the specifications/proofs of the verified ML-KEM code (commits `c1935b9cf8`, `5f6e3c2083`), with parts of the verification running in lax mode. The vendor disputes parts of this | Whether the reported issues are fixed in a current release; any independent audit |
| `aws-lc-rs` | **Not checked** on 2026-10-15 | Per Project Eleven (undated, about 2025): ML-KEM and ML-DSA (ML-DSA behind an unstable flag); post-quantum code not independently audited; FIPS work on the library. Rust bindings to the **AWS-LC C library**, so it needs a D-015 exception | Current version, FIPS validation scope, any newer audit |
| `liboqs` / `oqs` | **Not checked** | Per Project Eleven (about 2025): README says not recommended for production | Current README wording |
| RustCrypto `aes-gcm`, `chacha20poly1305` | Versions of the review **not stated** on NCC Group's public page | NCC Group implementation review for MobileCoin, December 2019 to February 2020 (report v1.0, 2020-02-12). Covers versions of that time, **not current releases** | Exact versions reviewed (in the PDF report); current-version assurance |
| `x25519-dalek` | **Not checked** | — | Audit history and version |
| `clatter` (PQNoise) | **2.1.0** (2025-12-21); 2.3.0 (2026-08-30) exists | 2.1.0 page: *"has not received any formal audit"*; single maintainer | 2.3.0 status |

**Reading of the evidence:** for the post-quantum libraries checked (`ml-kem` 0.3.2, `ml-dsa` 0.1.1, `clatter` 2.1.0), the maintainers state that no independent audit has been done. For the others, no independent audit was found in the sources checked. This is evidence for planning (M2-Q4), not a universal claim.

## 8. Provisional direction

1. **Use Approach A as the provisional leading candidate for drafting M2-02.**
   - Hybrid-WireGuard is the research basis, with PQNoise for the general pattern.
   - Every difference from the analysed design is marked **BLOCKED — needs G1 review**, including parameters, PSK handling, encodings, labels, cookies, roaming and versioning.
   - No statement in the specification will claim the analysed proofs cover Septuvo.
2. **Write M2-02 with named, replaceable parameter sets** for the ephemeral and static KEMs, so the M2-Q3 decision can follow the G1 reviewer's advice without redrafting.
3. **Keep every candidate a candidate** (D-010, D-011, D-012, D-024). Approach A would not use ML-DSA in the handshake; ML-DSA remains a candidate for signing enrollment data and updates.
4. **Defer the library choice** to after G1 (M2-Q4), based on the version-specific evidence above, re-checked at selection time.

## 9. Owner questions (M2) — status after owner review on 2026-10-15

| Q | Question | Status |
| --- | --- | --- |
| M2-Q1 | Use Hybrid-WireGuard as the basis for SVP's handshake? | **Accepted as the leading research basis only** — not an approved final construction. Its analysis does not prove Septuvo's version secure |
| M2-Q2 | Authenticate with static keys instead of handshake signatures? | **Acceptable direction to investigate**, provisional until G1; §4 describes how it would work and what is open (notably the PSK and KCI) |
| M2-Q3 | Static post-quantum KEM: ML-KEM-768 or Classic McEliece? | **Open.** §5 gives the sizes and trade-offs; M2-02 keeps parameters replaceable; the G1 reviewer will be asked |
| M2-Q4 | Library strategy | **Deferred to after G1**, using the version-specific evidence in §7 |

## Sources

- FIPS 203 (ML-KEM): <https://csrc.nist.gov/pubs/fips/203/final>
- FIPS 204 (ML-DSA): <https://csrc.nist.gov/pubs/fips/204/final>
- Classic McEliece parameter sizes (Open Quantum Safe): <https://openquantumsafe.org/liboqs/algorithms/kem/classic_mceliece>
- Classic McEliece considerations draft (June 2026): <https://www.ietf.org/archive/id/draft-josefsson-cfrg-mceliece-considerations-00.html>
- X-Wing draft -11: <https://www.ietf.org/archive/id/draft-connolly-cfrg-xwing-kem-11.html>
- CFRG hybrid KEMs draft -11: <https://www.ietf.org/archive/id/draft-irtf-cfrg-hybrid-kems-11.html>
- RFC 10024: <https://www.rfc-editor.org/rfc/rfc10024>
- Post Quantum Noise (CCS 2022): <https://eprint.iacr.org/2022/539>
- KEMTLS (CCS 2020): <https://eprint.iacr.org/2020/534/>
- Lafourcade et al., USENIX Security 2025: <https://www.usenix.org/conference/usenixsecurity25/presentation/lafourcade>
- NetBird on integrating Rosenpass: <https://netbird.io/knowledge-hub/how-we-integrated-rosenpass>
- Project Eleven, "The state of post-quantum cryptography in Rust": <https://blog.projecteleven.com/posts/the-state-of-post-quantum-cryptography-in-rust-the-belt-is-vacant>
- Symbolic Software on libcrux (12 Feb 2026): <https://symbolic.software/blog/2026-02-12-cryspen-response>
- RustCrypto `ml-kem`: <https://docs.rs/crate/ml-kem/latest>
- RustCrypto `ml-dsa`: <https://docs.rs/crate/ml-dsa/latest>
- NCC Group review of RustCrypto AEADs: <https://www.nccgroup.com/research/public-report-rustcrypto-aesgcm-and-chacha20pluspoly1305-implementation-review/>
- `clatter`: <https://docs.rs/crate/clatter/2.1.0>
- rustls defaults: <https://doc.servo.org/rustls/manual/_05_defaults/index.html>
