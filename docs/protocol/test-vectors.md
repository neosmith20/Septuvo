# SVP — test-vector formats, negative cases and scenarios (M2-04, draft)

**Status:** documentation only. **No vectors have been computed and none will be before G1** (D-021, option C). No generator, script or code accompanies this document.
- It defines *what* the vectors will contain and *how* they will be written down, so the G1 reviewer can check the plan before anything is produced.
- [conformance-plan.md](conformance-plan.md) says how they will be produced and checked after G1.

**License:** Apache-2.0 (open SVP specification, D-033).

| Evidence | State |
| --- | --- |
| Computed vectors | **NOT RUN.** Deferred to after G1 (D-021) |
| Vector formats (this document) | Draft |
| Negative cases and scenarios (this document) | Draft |
| Reviewer acceptance of this pre-G1 plan | **OPEN.** If the reviewer does not accept it, G1 stays blocked (D-021) |

## 1. File format (proposal)

**Encoding:**
- One vector file per category (§2), in **TOML**, UTF-8.
- Binary values are lowercase hexadecimal strings with no separators.
- Lengths are always stated alongside the value, so a truncated value is detectable.

**Every file starts with a header block:**

```toml
[meta]
spec_commit = "<full Git SHA of the reviewed specification>"   # the G1-reviewed commit
protocol_identifier = "<exact identifier string, handshake.md §8>"
parameter_set = "EPHEMERAL_KEM=…, STATIC_KEM=…, HASH=…, AEAD=…, DATA_AEAD=…"
produced_by = ["<implementation A name and version>", "<implementation B name and version>"]
status = "agreed" | "disputed"
```

**Rules:**
- A vector file is valid only for the `spec_commit` and `parameter_set` it names. **A change to either makes it void**; it is never edited by hand.
- `status = "agreed"` only when two independent implementations produced identical output ([conformance-plan.md](conformance-plan.md) §3).
- **Randomness in vectors** (ephemeral keys, KEM encapsulation coins, session identifiers, cookie nonces) is listed explicitly as an input. Implementations accept it only through a **test-only interface that does not exist in production builds** (§4).

## 2. Vector categories

| ID | Category | Inputs listed | Outputs listed | Source of truth |
| --- | --- | --- | --- | --- |
| V-PRIM | Primitives: X25519, ML-KEM, Classic McEliece, BLAKE2s, HKDF, ChaCha20-Poly1305, XChaCha20-Poly1305, AES-256-GCM | As in each standard's own test data | As in that test data | **The standards' published vectors** (RFC 7748, FIPS 203 / NIST ACVP, the Classic McEliece submission, RFC 7693, RFC 5869, RFC 8439 and so on). SVP does not produce its own primitive vectors; it checks that implementations pass the published ones |
| V-HS | Full handshake, one file per parameter set | Both parties' static keys, `psk`, `lbl1`–`lbl4`, all randomness, timestamp | **Every intermediate value** of [handshake.md](handshake.md) §6, steps 1–9 (`C`, `κ`, `H`, `shk1`–`shk3`, DH values), both complete messages byte for byte, `tki`, `tkr` | Spec commit |
| V-COOKIE | CookieReply | `Rm`, source address and port, nonce, the triggering InitHello | `cookie`, CookieReply bytes, the resulting `m2` | Spec commit |
| V-TR | Transport packets | Key, direction, `receiver`, `counter`, plaintext | Padded plaintext, complete packet bytes; then the next counter | Spec commit |
| V-FP | Fingerprints | Server public keys (`Ksig`, `Scr`, `Spqr`), identifier | Fingerprint bytes and the displayed form | Spec commit (after B-02 is settled) |
| V-ENR | Enrollment binding | TLS exporter value, invitation secret | Invitation proof, confirmation code (handshake.md §10.1) | Spec commit (after B-12 is settled) |

**Order of production after G1:** V-PRIM checks first, then V-HS, V-COOKIE, V-TR, and finally V-FP and V-ENR. V-FP and V-ENR depend on BLOCKED items that may change.

## 3. Negative cases

Each case is a **valid vector with one change**, plus the required outcome. In every case **nothing is sent back except where stated, no state changes, and the stated reason code is counted** ([state-machine.md](state-machine.md) §6).

### 3.1 Parsing (all message types)

| ID | Change | Required outcome |
| --- | --- | --- |
| N-P01 | Datagram one byte shorter than the exact size for its `type` | Drop, `malformed`, before any cryptography |
| N-P02 | One byte longer | Drop, `malformed` |
| N-P03 | Unknown `type` (each of 0x00, 0x05–0xFF) | Drop, `malformed` |
| N-P04 | `version` other than the configured one | Drop, `malformed` |
| N-P05 | Non-zero `reserved` | Drop, `malformed` |
| N-P06 | Zero-length datagram; a datagram larger than the maximum transport size | Drop, `malformed`; no allocation based on the length |
| N-P07 | Transport packet shorter than 32 bytes | Drop, `malformed` |

### 3.2 InitHello (server side)

| ID | Change | Required outcome |
| --- | --- | --- |
| N-I01 | `Epqi` fails the FIPS 203 encapsulation-key check | Drop, `bad_key` |
| N-I02 | `Eci` is a low-order point giving an all-zero DH result | Drop, `bad_key` (if B-10 is adopted) |
| N-I03 | One bit flipped in `ct1enc` | Decapsulation result changes; `static` fails to decrypt; drop, `decap_failed` or `bad_mac`. No difference visible on the wire |
| N-I04 | `static` names an unenrolled device | Drop, `unknown_device` |
| N-I05 | `static` names a revoked device | Drop, `revoked` |
| N-I06 | Correct device, wrong `psk` | `m1` fails; drop, `bad_mac` |
| N-I07 | One bit flipped in `time` | Drop, `bad_mac` |
| N-I08 | Valid InitHello replayed (same timestamp) | Drop, `stale_timestamp`; the existing `pending` slot is **not** replaced |
| N-I09 | Valid InitHello with an older timestamp | Drop, `stale_timestamp` |
| N-I10 | Device using a different protocol identifier | `static` fails to decrypt; drop. No downgrade, no reply |
| N-I11 | Classical key of device A with post-quantum key of device B (mix-and-match) | Fails the static-key binding; drop |
| N-I12 | Under load, `m2` missing or wrong | Exactly one 64-byte CookieReply; no decapsulation performed (counted) |
| N-I13 | Under load, valid `m2` from a source whose bucket is empty | Drop, `rate_limited` |

### 3.3 RespHello (device side)

| ID | Change | Required outcome |
| --- | --- | --- |
| N-R01 | `sidi` does not match the outstanding attempt | Drop; keep waiting |
| N-R02 | RespHello for a replaced attempt (late) | Drop (state-machine D3) |
| N-R03 | One bit flipped in `ct2`, `ct3enc` or `empty` | Drop, `decap_failed` or `bad_mac`; keep waiting |
| N-R04 | RespHello produced with a different server static key (impostor) | `empty` fails; drop; status never reaches stage 2 |
| N-R05 | Valid RespHello delivered twice | The second is dropped; no second key change |

### 3.4 CookieReply (device side)

| ID | Change | Required outcome |
| --- | --- | --- |
| N-C01 | Associated data does not match the `m1` of any InitHello the device sent | Drop |
| N-C02 | Encrypted with a key derived from the wrong server identity | Drop |
| N-C03 | CookieReply received while not in `HANDSHAKE_SENT` | Drop |

### 3.5 Transport

| ID | Change | Required outcome |
| --- | --- | --- |
| N-T01 | Unknown `receiver` | Drop |
| N-T02 | Duplicate `counter` | Drop (replay) |
| N-T03 | `counter` older than the window | Drop |
| N-T04 | One bit flipped in the header (associated data) | AEAD fails; drop; **the replay window is not advanced** |
| N-T05 | One bit flipped in the ciphertext or tag | As N-T04 |
| N-T06 | Valid packet whose inner source address is not assigned to the device | Drop after decryption (B-22) |
| N-T07 | Valid packet to a destination outside the device's scope | Drop (S-09) |
| N-T08 | Packet under `PENDING` keys sent **by the server** | Must never be produced: a test that an implementation cannot emit it |
| N-T09 | Packet under keys older than `REJECT_AFTER_TIME` | Drop |
| N-T10 | Integrity limit of failed decryptions reached for a key | Key retired; new handshake (key-lifecycle §4) |

## 4. Deterministic randomness: test-only rule

Vectors need fixed randomness, but **production code must never accept injected randomness** (AGENTS.md: no static nonces, no insecure random-number generators). So, after G1:
- The deterministic interface exists only in a **separate test-only build configuration or crate**, which release builds cannot enable.
- A CI check fails the build if a release artifact contains it.
- Production code obtains randomness only from the operating system's generator (ASM-02).
- The design of this separation is reviewed as part of the first post-G1 work order. It is listed here so the reviewer sees it before G1.

## 5. State and replay scenarios

Each scenario is a sequence of events with the required states, using the transition numbers of [state-machine.md](state-machine.md). They run in a simulated clock and network (after G1), and later as lab tests (G2).

| ID | Scenario | Required result |
| --- | --- | --- |
| SC-01 | Normal connect | D1 → D2; S1 → S3; status reaches stage 2 on both sides only after D2 / S3 |
| SC-02 | RespHello lost | D4 retransmits with **new** ephemerals; server S1 replaces `pending`; then SC-01 |
| SC-03 | Server unreachable for longer than `REKEY_ATTEMPT_TIME` | D5 → `FAILED`; the kill switch stays in force; status "server not answering" |
| SC-04 | Time-based rekey with traffic in both directions | D6 → D8; no packet is accepted twice; no gap in delivery beyond one round trip |
| SC-05 | Rekey when only the server sends (download only) | D7 triggers; the session never reaches `REJECT_AFTER_TIME` while in use |
| SC-06 | Packet-count rekey (`REKEY_AFTER_MESSAGES` set low in the simulation) | Rekey happens before the limit; the counter never reaches the reject limit |
| SC-07 | Server restart mid-session | S8; device re-handshakes; no nonce is reused (new keys) |
| SC-08 | Device restart mid-session | As SC-07 from the other side |
| SC-09 | Replay of a recorded InitHello after a server restart | Dropped if the timestamp table was persisted (B-11); otherwise the documented disruption only, never usable keys |
| SC-10 | Revocation during an active session | S7; all keys erased at once; the next InitHello fails (`revoked`) |
| SC-11 | Device roams to a new address | Server follows after an authenticated packet (§7 of wire-format); if B-21 is adopted, path validation and the A1 byte limit apply |
| SC-12 | On-path redirection attempt (copied packet from another address) | Documented outcome per the B-21 decision |
| SC-13 | Reordering within the replay window | All packets accepted once; none twice |
| SC-14 | Handshake flood from spoofed addresses | Only CookieReplies, each ≤ the received size; existing sessions unaffected (P5) |
| SC-15 | Handshake flood from real addresses | Budgets apply; new handshakes may be refused; existing sessions unaffected |
| SC-16 | Clock set backwards on a device | That device's handshakes fail until its clock passes; others unaffected |
| SC-17 | Different protocol identifiers on device and server | No session; device reports "server not answering"; no fallback attempted |

## 6. Future property tests and fuzzing (requirements; after G1)

- **Parsers:** fuzzed with arbitrary datagrams. They must never panic, read out of bounds or allocate based on attacker-controlled lengths (T1).
- **Properties to state and test:**
  - a nonce (key, counter) is never used twice;
  - the replay window only advances after authentication;
  - the server never sends transport data under `PENDING` keys;
  - bytes sent to an unvalidated address never exceed bytes received from it;
  - no state transition outside [state-machine.md](state-machine.md) is reachable.
- Concurrency: packet workers cannot race counters or slots (T1).

All of this is **NOT RUN** and cannot run before G1. It is the plan the reviewer is asked to accept.
