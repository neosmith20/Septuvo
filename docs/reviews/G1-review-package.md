# G1 review package: Septuvo Protocol (SVP) specification

**For:** a qualified, independent reviewer of cryptographic protocols.
**From:** Alex, owner of the Septuvo project (GitHub `neosmith20`).
**Status:** ready to send. **G1 has not passed. No reviewer has been engaged yet.** The search and its results are recorded in [G1.md](G1.md).

**License:** the specification is Apache-2.0 (D-033).

## 1. What we are asking for

A **written design review of the SVP specification at one fixed commit**, before any protocol or cryptographic code is written. Septuvo's rules forbid such code until this review exists and the owner has approved gate G1 on the strength of it (AGENTS.md; [ROADMAP.md](../ROADMAP.md) M2).

| Item | Value |
| --- | --- |
| **Commit to review** | `c82152ac1691fa9854d24474950f11847b85cc43` (main, 2026-10-16) |
| Repository | <https://github.com/neosmith20/Septuvo> |
| Size | About 20,000 words across the protocol documents and the threat model |
| Implementation | **None exists.** The only code in the repository is non-cryptographic lab tooling (kill switch and status display, track N1), which is outside this review |

If the specification changes while the review is in progress, we will **not** change the commit under review without agreeing it with the reviewer first.

## 2. What SVP is, in brief

- SVP is the protocol of **Septuvo VPN**, a self-hosted VPN: a person's own server and their own devices, with no accounts, cloud service or telemetry.
- The handshake **adapts Hybrid-WireGuard** (Lafourcade et al., "A Tale of Two Worlds, a Formal Story of WireGuard Hybridization", USENIX Security 2025):
  - a two-message, WireGuard-style handshake authenticated by **static keys**, with no handshake signatures;
  - hybrid by construction: X25519 together with ML-KEM ephemeral keys and a post-quantum static KEM (Classic McEliece in the analysed design).
- We transcribed that design and kept to it. **Every place where Septuvo differs, or where we filled a gap the paper leaves open, is listed as a "BLOCKED" item (B-00 to B-24).** Those items are the main object of the review.
- We do **not** claim that the paper's analysis covers Septuvo's protocol.

## 3. Documents, in reading order (all at the commit above)

| # | Document | What it contains |
| --- | --- | --- |
| 1 | [threats.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/spec/threats.md) | Threat model: adversaries ADV-01 to ADV-12, assumptions, trust boundaries |
| 2 | [identity-and-privacy.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/spec/identity-and-privacy.md) | Identity, enrollment, revocation and privacy requirements |
| 3 | [candidate-analysis.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/protocol/candidate-analysis.md) | Why this construction; packet sizes for each parameter choice; library evidence |
| 4 | [handshake.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/protocol/handshake.md) | Roles, keys, messages, key schedule, authentication, downgrade, enrollment. **BLOCKED register B-00 to B-14 in §12** |
| 5 | [state-machine.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/protocol/state-machine.md) | Every state, transition and timer; fail-closed rules |
| 6 | [wire-format.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/protocol/wire-format.md) | Byte layouts, replay window, MTU, Septuvo's own denial-of-service and anti-amplification design, roaming. **BLOCKED B-15 to B-24 in §10** |
| 7 | [key-lifecycle.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/protocol/key-lifecycle.md) | All key material, epochs, usage limits, rotation, crash recovery |
| 8 | [test-vectors.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/protocol/test-vectors.md) | Planned vector formats, negative cases, scenarios (no computed vectors, by policy) |
| 9 | [conformance-plan.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/protocol/conformance-plan.md) | **Checklist mapping P-01 to P-14** and the conformance method |
| — | [PROTOCOL_REVIEW_CHECKLIST.md](https://github.com/neosmith20/Septuvo/blob/c82152ac1691fa9854d24474950f11847b85cc43/docs/PROTOCOL_REVIEW_CHECKLIST.md) | The 14 items the review should cover |

## 4. Scope

**In scope:**
- checklist items P-01 to P-14, with answers mapped in conformance-plan §1;
- all BLOCKED items;
- whether our transcription of the source design is faithful.

**Out of scope:**
- implementation (none exists);
- the N1 lab tooling;
- user-interface and product decisions;
- licensing;
- the security of the underlying primitives themselves (ML-KEM, Classic McEliece, X25519 and so on) beyond whether they are used correctly.

## 5. The questions we most need answered

In priority order. Each names the BLOCKED items concerned.

| # | Question | Items |
| --- | --- | --- |
| RQ-1 | **Transcription.** Is our transcription of Hybrid-WireGuard (handshake.md §5, §6, §9.2) faithful to the paper? Our InitHello layout comes out **4 bytes smaller** than the paper's Table 8 total; where are they? | B-00 |
| RQ-2 | **Parameter sets.** Which ephemeral and static KEM parameter sets should version 1 use? The analysed configuration is ML-KEM-512 + Classic McEliece level 3, and it avoids fragmentation. ML-KEM-768 throughout is our original candidate, but it needs fragmentation and moves away from the analysed design. Is ML-KEM-512 for the *ephemeral* exchange acceptable? | B-03 |
| RQ-3 | **Pre-shared key and KCI.** Some analysed properties assume a secret PSK. Is a random per-device PSK delivered at enrollment the right approach, and does Septuvo's version keep KCI resistance? | B-04 |
| RQ-4 | **Transcript hash.** As transcribed, the static-KEM ciphertexts `ct1`/`ct3` enter only the chaining key, not the transcript hash. Is that acceptable for the chosen KEMs' binding properties? | B-00, B-05 |
| RQ-5 | **Enrollment.** Is TLS 1.3 with hybrid key exchange (RFC 10024), a raw-public-key signing identity and exporter-bound proofs (handshake.md §10.1) a sound way to deliver the static keys and PSK and bind them to the invitation? | B-02, B-12 |
| RQ-6 | **Denial of service.** Is the cookie, budget and anti-amplification design adequate? Is the residual risk acceptable: a botnet can delay *new* handshakes, but not existing sessions? | B-06, B-19, B-20 |
| RQ-7 | **Data cipher limits.** AES-256-GCM would need packet-count rekeying every few seconds on fast links. Should version 1 prefer ChaCha20-Poly1305, and what exact per-key limits apply at our packet sizes? | B-14, B-17 |
| RQ-8 | **Downgrade.** Is binding the full parameter set into the first key-schedule step, with no in-band negotiation and the identifier pinned at enrollment, sufficient? | B-07, B-09 |
| RQ-9 | **Roaming.** Is QUIC-style path validation worth adding to close the on-path redirection/reflection weakness WireGuard accepts? | B-21 |
| RQ-10 | **Replay after restart.** Must the timestamp table be persisted before replying, or is the documented disruption acceptable? | B-11 |
| RQ-11 | **Formal analysis (P-14).** Should the paper's SAPIC+/ProVerif/Tamarin models be extended to cover Septuvo's changes before implementation? If so, which changes first? | P-14 |
| RQ-12 | **Pre-G1 evidence.** By owner decision D-021, no test vectors are computed before G1. Is the documentation-only plan (test-vectors.md, conformance-plan.md) sufficient for this stage? | P-13 |

## 6. What we need back

A written report that:
1. names the reviewer(s), their relevant qualifications, and any conflict of interest;
2. names the **exact commit** reviewed;
3. lists findings with a severity (for example critical, high, medium, low, informational) and a recommended fix;
4. answers RQ-1 to RQ-12, or says why a question cannot be answered;
5. states the **residual risks** the reviewer sees;
6. says whether, in the reviewer's opinion, **implementation of a lab prototype may begin**, and under which conditions (the checklist's acceptance rule).

**Publication.** We would like to publish the report in the repository (`docs/reviews/`), with the reviewer's permission and in any form they prefer, including redacted or after fixes.

## 7. How findings are handled

- Each finding is answered by a small, separate change to the specification, linked to the finding.
- A material change reopens review of that part. The reviewer is asked to confirm fixes on the new commit.
- **G1 passes only by the owner's explicit decision, recorded on GitHub, after the reviewer's written sign-off on the exact commit.** The project's agents cannot pass G1, and the reviewer is not asked to approve anything on the owner's behalf.

## 8. Disclosure

The specification was drafted with AI assistance (Anthropic's Claude), directed and reviewed by the owner. The project's rules require an independent human expert review for exactly this reason: no AI-written analysis is treated as evidence of security.

## 9. Contact

Alex, through GitHub (`neosmith20`) or the contact details given in the covering message.
