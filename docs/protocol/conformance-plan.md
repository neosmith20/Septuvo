# SVP — conformance plan and review-checklist mapping (M2-04, draft)

**Status:** documentation only (D-021). Nothing here has been run.
- It says how a future implementation will be shown to match the reviewed specification.
- It maps every item of [PROTOCOL_REVIEW_CHECKLIST.md](../PROTOCOL_REVIEW_CHECKLIST.md) (P-01 to P-14) to the section that answers it, or records it as an **OPEN** finding.

**License:** Apache-2.0 (open SVP specification, D-033).

## 1. Checklist mapping (P-01 to P-14)

"Draft" means a written answer exists for the reviewer. **No item is reviewed or approved; G1 has not passed.**

| Item | What reviewers must see | Where it is answered | State | Open points |
| --- | --- | --- | --- | --- |
| P-01 | Threat and trust model | [threats.md](../spec/threats.md); [SECURITY_MODEL.md](../SECURITY_MODEL.md); [handshake.md](handshake.md) §2 | Draft | Server-host compromise (ADV-08) is out of the protocol's scope; the reviewer should confirm that framing |
| P-02 | Version and wire format | [wire-format.md](wire-format.md) §1–§4 | Draft | B-15, B-16; the 4-byte InitHello difference from the paper (B-00) |
| P-03 | Identity, enrollment, revocation, recovery | [handshake.md](handshake.md) §2, §3, §10, §10.1, §11; [key-lifecycle.md](key-lifecycle.md) §5; [identity-and-privacy.md](../spec/identity-and-privacy.md) | Draft | **Enrollment construction B-12**; key replacement B-13; server key migration OPEN |
| P-04 | Hybrid handshake | [handshake.md](handshake.md) §5, §6, §7, §9 | Draft | **Parameter sets B-03 (M2-Q3)**; PSK B-04; transcription B-00 |
| P-05 | Transcript and KDF | [handshake.md](handshake.md) §6, §8 | Draft | Labels B-07; HKDF conventions B-05; whether `ct1`/`ct3` must enter the transcript hash (§6 note) |
| P-06 | Cipher suites and negotiation | [handshake.md](handshake.md) §4, §8; [wire-format.md](wire-format.md) §4 | Draft | `DATA_AEAD` choice B-17; no in-band negotiation (by design) |
| P-07 | Packet numbering and nonces | [wire-format.md](wire-format.md) §4; [key-lifecycle.md](key-lifecycle.md) §4, §6 | Draft | GCM limits B-17 |
| P-08 | Replay and state | [wire-format.md](wire-format.md) §3, §4.1, §7; [state-machine.md](state-machine.md) | Draft | Window B-18; roaming B-21; timestamp persistence B-11 |
| P-09 | Rotation and lifetime | [key-lifecycle.md](key-lifecycle.md) §2–§5; [state-machine.md](state-machine.md) §1 | Draft | Timer values B-14 |
| P-10 | MTU and fragmentation | [wire-format.md](wire-format.md) §5 | Draft | Fragmentation B-23 (only if M2-Q3 needs it); outer path-MTU discovery OPEN |
| P-11 | Resource protection | [wire-format.md](wire-format.md) §6 | Draft | B-06, B-19, B-20; **residual botnet risk stated** (§6.5); budget values need G2 measurement |
| P-12 | Error semantics | [state-machine.md](state-machine.md); [handshake.md](handshake.md) §9.4; [wire-format.md](wire-format.md) §8 | Draft | — |
| P-13 | Wire fixtures | [test-vectors.md](test-vectors.md); this document | Draft plan; **computed vectors NOT RUN** (D-021) | The reviewer must accept the plan as sufficient pre-G1 evidence |
| P-14 | Protocol analysis | **OPEN finding.** No review record exists yet. The only formal analysis is of the source design (Lafourcade et al. 2025) | **OPEN** | Whether to extend the paper's SAPIC+ (ProVerif/Tamarin) models to Septuvo's changes is a question for the reviewer (M2-05) |

**Summary for the reviewer:**
- Every item has a draft answer except P-14, which is open.
- The BLOCKED register is B-00 to B-14 in [handshake.md](handshake.md) §12, and B-15 to B-24 in [wire-format.md](wire-format.md) §10.

## 2. What "conformance" will mean (after G1)

An implementation conforms to SVP at a reviewed `spec_commit` when all of the following hold:
1. It reproduces every **V-HS, V-COOKIE, V-TR, V-FP and V-ENR** vector for its parameter set **exactly** ([test-vectors.md](test-vectors.md) §2), and passes the published **V-PRIM** vectors of every primitive it uses.
2. It gives the required outcome for **every negative case** N-P01 to N-T10 ([test-vectors.md](test-vectors.md) §3), including "no reply" and "no state change".
3. It follows every **scenario** SC-01 to SC-17 in a simulated network ([test-vectors.md](test-vectors.md) §5).
4. It **interoperates** with an independent implementation (§3) in the lab.
5. Its fuzzing and property tests ([test-vectors.md](test-vectors.md) §6) run for the agreed time without failures.

## 3. Independent implementations

- Vectors are trusted only when **two independent implementations agree**:
  - the Septuvo Rust implementation;
  - a second, minimal implementation written **from the specification alone** by a different author (person or agent) who has not read the first implementation's code.
- The second implementation exists only to cross-check vectors and is never shipped. Its language and licence are chosen in a post-G1 work order.
- **A disagreement is treated as a specification defect first.** The text is clarified in a pull request, and a material change reopens review of that part (AGENTS.md, G1 rules). Code is then fixed to the clarified text, never the other way round.
- **Primitive libraries:** both implementations may use standard libraries for primitives. If they use the **same** library, a V-PRIM pass does not test that library independently, and this is recorded.

## 4. Traceability

| Requirement | Specification | Test (planned) |
| --- | --- | --- |
| S-01 Authenticated key agreement | handshake §5–§9 | V-HS, N-I03 to N-I11, N-R03, N-R04 |
| S-02 Quantum resistance (hybrid) | handshake §4, §6 | V-HS for each parameter set; reviewer assessment |
| S-03 Key lifecycle | key-lifecycle | SC-04 to SC-10, N-T09, N-T10 |
| S-04 AEAD safety | wire-format §4; key-lifecycle §4 | V-TR, N-T04, N-T05; property tests |
| S-05 Downgrade safety | handshake §8 | N-I10, SC-17 |
| S-06 Replay and endpoint changes | wire-format §4.1, §7 | N-I08, N-T02, N-T03, SC-11 to SC-13 |
| S-07 Denial of service | wire-format §6 | N-I12, N-I13, SC-14, SC-15 |
| S-09 Authorization | wire-format §4 step 6 | N-T06, N-T07 |
| S-10 Enrollment | handshake §10, §10.1 | V-FP, V-ENR (after B-02, B-12) |
| S-13 Accurate status | state-machine §4 | SC-01, SC-03, N-R04 |

These rows will be added to [TRACEABILITY.md](../TRACEABILITY.md) and [TEST_MATRIX.md](../TEST_MATRIX.md) (T1) when the tests exist. Until then they are **NOT RUN**.

## 5. Order of work after G1 (for planning only)

Nothing below is authorized by this document. Each step needs the post-G1 work orders (M3 and later):
1. Resolve or accept every BLOCKED item, and fix the parameter set (M2-Q3).
2. Approve dependencies (D-015).
3. Set up the test-only randomness separation ([test-vectors.md](test-vectors.md) §4).
4. Implement the parsers with fuzzing first.
5. Build both implementations; produce and agree V-HS, then the other vectors.
6. Run the negative cases and scenarios in simulation.
7. Run the G2 lab tests.

**Stop point of M2:** after this document, the specification set is complete for review. The next step is **M2-05** (review package and outreach), which the owner starts personally ([work-orders/M2.md](../work-orders/M2.md)).
