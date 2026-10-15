# SVP — identity and handshake specification (M2-02, draft)

**Status:** draft for owner review and for the G1 specialist review. **Nothing in this document is approved, and it is not an implementation guide.**
- It writes down the handshake SVP would use if the provisional direction of [candidate-analysis.md](candidate-analysis.md) §8 holds: **Approach A**, with Hybrid-WireGuard as the research basis.
- D-010, D-011, D-012 and D-024 remain **candidates**; D-013 remains **blocked**. No code, no computed vectors (D-021).
- The states, transitions and timers are in [state-machine.md](state-machine.md). Byte layouts, cookies, anti-amplification, packet numbers and key epochs are M2-03.

**License:** Apache-2.0 (open SVP specification, D-033).

> **Scope of any security statement here.** The analysis by Lafourcade et al. (USENIX Security 2025) covers *their* protocol exactly as specified: a symbolic (Dolev-Yao) analysis with a secret pre-shared key and secure primitives assumed. It does **not** cover:
> - this document;
> - denial of service, cookies, timestamps, rekeying or the data phase (the paper says these were not analysed);
> - computational security;
> - any Septuvo change.
>
> Every difference from the analysed design is marked **BLOCKED — needs G1 review** and listed in §12. The reviewer should read §12 first.

## 1. How to read this document

| Mark | Meaning |
| --- | --- |
| **[HWG]** | Transcribed from the analysed design (Lafourcade et al. 2025, Tables 5–7, and Appendix A). The transcription is ours. **The reviewer must check it against the paper** (§12, B-00) |
| **[WG]** | Taken from the WireGuard whitepaper. Not covered by the Hybrid-WireGuard analysis unless that paper says so |
| **[STD]** | Required by a cited standard (FIPS 203, RFC 7748, RFC 5869 and so on) |
| **BLOCKED B-nn** | Septuvo-specific. Not taken verbatim from a reviewed source. Must be reviewed at G1 before anything depends on it |

Notation follows the paper:
- `‖` is concatenation, `∅` the empty string.
- `KDFn(k, x)[j]` is the j-th of n outputs.
- Subscript `i` is the initiator (the device) and `r` the responder (the server).
- `S` is a static public key and `s` the matching private key; `E`/`e` are ephemeral; `c` means classical (X25519) and `pq` post-quantum.

## 2. Roles and trust roots

| Item | Specification | Source / status |
| --- | --- | --- |
| **Initiator** | Always the **device**. The server never starts a handshake towards a device | **BLOCKED B-01.** WireGuard lets either side initiate. Restricting the role removes simultaneous-initiation handling but is a Septuvo choice |
| **Responder** | Always the **Septuvo Server** | B-01 |
| **Device trust root** | The server identity pinned at enrollment (identity-and-privacy §2, EN-6). The device holds the server's full static public keys `Scr`, `Spqr`, checked against the invitation fingerprint. A different server identity is refused, never accepted automatically (RV-5) | M1 requirement; fingerprint construction **BLOCKED B-02** |
| **Server trust root** | The server's own list of enrolled devices: each record holds `Sci`, `Spqi`, the device's PSK (§3.3), its scope and its revocation state | M1 requirement (identity-and-privacy §2, RV-1, RV-2) |
| **No other authority** | No certificate authority, account or directory in version 1 (D-007) | M1 |

## 3. Keys

### 3.1 Long-term (static) identity keys

| Key | Algorithm | Where generated | Source / status |
| --- | --- | --- | --- |
| `(sci, Sci)`, `(scr, Scr)` | X25519 | On the machine, from the OS random-number generator (ID-1, ASM-02) | [HWG] + [STD] RFC 7748 |
| `(spqi, Spqi)`, `(spqr, Spqr)` | Parameter `STATIC_KEM` (§4) | Same | [HWG] uses Classic McEliece level 3. The choice is open (M2-Q3), **BLOCKED B-03** |

- **Identity** = the pair (classical public key, post-quantum public key), never one alone.
- **Fingerprint** shown to people = a hash of both public keys. For the *server*, it also covers the enrollment signing key and the protocol identifier (§10.1). The hash, input encoding and truncation are **BLOCKED B-02**.

### 3.2 Ephemeral keys (fresh for every handshake)

| Key | Algorithm | Source |
| --- | --- | --- |
| `(eci, Eci)`, `(ecr, Ecr)` | X25519 | [HWG] |
| `(epqi, Epqi)` | Parameter `EPHEMERAL_KEM` (§4) | [HWG] uses ML-KEM level 1 (ML-KEM-512). Open (M2-Q3), **BLOCKED B-03** |
| KEM encapsulation randomness | Fresh from the OS random-number generator for every encapsulation | [HWG] (probabilistic encapsulation, the paper's fix) + [STD] FIPS 203 |
| Session IDs `sidi`, `sidr` | 32 random bits | [HWG]; uniqueness rules are M2-03 |

### 3.3 Pre-shared key (PSK)

- In the analysed design, `psk` keys the first MAC (`m1`) and enters the key derivation twice. Some properties hold **only if `psk` is secret** ([candidate-analysis.md](candidate-analysis.md) §4).
- **Proposal (BLOCKED B-04, owner question M2-Q6):** a fresh, random, 32-byte `psk` per device, created during enrollment and delivered over the enrollment channel (§10). The server and that device both store it as secret key material (KS-1 to KS-5).
- **Alternative:** a fixed all-zero `psk`, as WireGuard uses when no PSK is configured. The reviewer would then have to state which analysed properties still hold.

## 4. Parameter sets (named and replaceable)

The handshake is written against **named parameters**, so the M2-Q3 decision can follow the G1 reviewer's advice without redrafting. Exactly **one parameter set is valid per protocol identifier** (§8).

| Parameter | Analysed design [HWG] | Septuvo candidates | Status |
| --- | --- | --- | --- |
| `DH` | X25519 | X25519 | [HWG] |
| `EPHEMERAL_KEM` | ML-KEM level 1 | ML-KEM-512 or ML-KEM-768 | **BLOCKED B-03** (M2-Q3) |
| `STATIC_KEM` | Classic McEliece level 3 | mceliece460896, mceliece348864 or ML-KEM-768 | **BLOCKED B-03** (M2-Q3) |
| `HASH` | BLAKE2s | BLAKE2s, or SHA-256 to match candidate D-024 | **BLOCKED B-05** (M2-Q5) |
| `MAC` (keyed hash for `m1`, `m2`) | Keyed BLAKE2s | As `HASH` | B-05 |
| `KDF` | HKDF (RFC 5869) with HMAC-`HASH` | HKDF (candidate D-024) | B-05 |
| `AEAD` (handshake fields) | ChaCha20-Poly1305 | ChaCha20-Poly1305 for the handshake; the data-phase cipher (candidate D-010) is M2-03 | B-05 |
| `SE` (unauthenticated stream encryption of KEM ciphertexts) | ChaCha20 | ChaCha20 | [HWG] |

**Recommendation (M2-Q5):** keep the analysed symmetric primitives *inside the handshake* (BLAKE2s, HKDF-HMAC-BLAKE2s, ChaCha20-Poly1305, ChaCha20). Changing them is a substitution the reviewer would have to re-check, and it brings no security benefit we can show. The data-phase cipher remains a separate decision (D-010, M2-03).

## 5. Messages

Field order is as in the analysed design. Byte widths, the values of `type` and reserved fields are **M2-03**.

### 5.1 InitHello (device → server) [HWG]

```
inner    = type ‖ sidi ‖ Eci ‖ Epqi ‖ ct1enc ‖ static ‖ time
InitHello = inner ‖ m1 ‖ m2
```

| Field | Content |
| --- | --- |
| `ct1enc` | `SE.Enc(KDF1(∅, DH(eci, Scr)), 0, ct1)`, where `(ct1, shk1) = STATIC_KEM.Encaps(Spqr)` |
| `static` | `AEAD.Enc(κ3, 0, H3, HASH(Sci ‖ Spqi))` |
| `time` | `AEAD.Enc(κ4, 0, H4, timestamp)`. The timestamp format and the replay rule are §9 |
| `m1` | `MAC(HASH(lbl3 ‖ psk), inner)` |
| `m2` | `MAC(cookie, inner ‖ m1)`, where `cookie` is all zero unless a cookie was received. **The cookie mechanism is M2-03, BLOCKED B-06**; the paper did not analyse it |

### 5.2 RespHello (server → device) [HWG]

```
inner     = type ‖ sidr ‖ sidi ‖ Ecr ‖ ct2 ‖ ct3enc ‖ empty
RespHello = inner ‖ m1 ‖ m2
```

| Field | Content |
| --- | --- |
| `ct2` | `(ct2, shk2) = EPHEMERAL_KEM.Encaps(Epqi)` |
| `ct3enc` | `SE.Enc(KDF1(∅, DH(sci, ecr)), 0, ct3)`, where `(ct3, shk3) = STATIC_KEM.Encaps(Spqi)` |
| `empty` | `AEAD.Enc(κ9, 0, H9, ∅)`. Authenticates the whole exchange to the device (key confirmation, §7) |
| `m1`, `m2` | As in InitHello |

### 5.3 Messages specified elsewhere

- **CookieReply:** M2-03 (B-06).
- **First transport packet:** confirms the keys to the server (§7). Its format is M2-03.

### 5.4 Estimated sizes

[candidate-analysis.md](candidate-analysis.md) §5 has the size estimates. With the analysed parameters, InitHello is 1156 B and RespHello 1064 B, including IPv6 and UDP headers.

## 6. Key schedule and transcript [HWG]

**Labels.**
- `lbl1`, `lbl2` and `lbl3` are public constants. In WireGuard they are the construction name, an identifier string and `"mac1----"`.
- Septuvo's strings are **BLOCKED B-07**. `lbl1` and `lbl2` must carry the protocol identifier of §8.

**Diffie-Hellman terms.**
- Each side computes the same DH value from its own private key and the other's public key, for example `DH(eci, Scr) = DH(scr, Eci)`.

**Chaining key, symmetric keys and transcript hash.** Steps 1–5 are computed by both sides from InitHello; steps 6–9 from RespHello.

| Step | Chaining key `C` (and symmetric key `κ`) | Transcript hash `H` | Uses |
| --- | --- | --- | --- |
| 1 | `C1 = HASH(lbl1)` | `H1 = HASH(C1 ‖ lbl2)` | Protocol identity |
| 2 | `C2 = KDF1(C1, Eci ‖ Epqi)` | `H2 = HASH(H1 ‖ HASH(Scr ‖ Spqr))` | Responder's static keys bound into `H` |
| 3 | `(C3, κ3) = KDF2(C2, DH(eci, Scr) ‖ shk1)` | `H3 = HASH(H2 ‖ Eci ‖ Epqi)` | Ephemeral-static: classical and post-quantum |
| 4 | `(C4, κ4) = KDF2(C3, DH(sci, Scr) ‖ HASH(Spqi ‖ Spqr) ‖ psk)` | `H4 = HASH(H3 ‖ static)` | Static-static; both post-quantum static keys; PSK |
| 5 | — | `H5 = HASH(H4 ‖ time)` | |
| 6 | `C6 = KDF1(C4, Ecr ‖ ct2)` | `H6 = HASH(H5 ‖ Ecr ‖ ct2)` | |
| 7 | `C7 = KDF1(C6, DH(eci, ecr) ‖ shk2)` | — | Ephemeral-ephemeral: forward secrecy |
| 8 | `C8 = KDF1(C7, DH(sci, ecr) ‖ shk3)` | — | Static-ephemeral: classical and post-quantum |
| 9 | `(C9, τ, κ9) = KDF3(C8, psk)` | `H9 = HASH(H6 ‖ τ)` | |
| Session | `(tki, tkr) = KDF2(C9, ∅)`. `tki` protects device→server traffic, `tkr` server→device | | Data phase: M2-03 |

**Transcription notes for the reviewer (B-00):**
- **Static-key binding.** Hashing `Scr ‖ Spqr` into `H2`, `HASH(Spqi ‖ Spqr)` into `C4`, and encrypting `HASH(Sci ‖ Spqi)` is the paper's fix: it stops a classical key of one party being combined with another party's post-quantum key ([candidate-analysis.md](candidate-analysis.md) §4).
- **What the transcript hash contains.** As transcribed, `ct1` and `ct3` (and their encrypted forms) are **not** hashed into `H`. They enter only through `shk1` and `shk3` in `C`. `ct2` is hashed into `H`. Please confirm against the paper, and say whether the chosen KEMs' binding properties make this acceptable for Septuvo's parameter sets.
- **Empty-key KDF.** `KDF1(∅, ·)` for the `SE` keys, and the fixed nonce `0` for `SE` and `AEAD`, are as written in the paper. Each key is used once per handshake.
- **HKDF details.** The salt, info and output-length conventions (WireGuard uses the chaining key as the HMAC key, with no info string) are **BLOCKED B-05** until the reviewer confirms them for the chosen `HASH`.

## 7. Authentication and key confirmation

| Direction | When it is established | Source / status |
| --- | --- | --- |
| **Server → device** | When the device decrypts `empty` (step 9) without error. Only a holder of `scr` and `spqr` can produce it: they are needed for `DH(eci, Scr)`, `DH(sci, Scr)` and `shk1`. **Explicit key confirmation** | [HWG] message agreement |
| **Device → server** | The server knows **which device** is claiming after decrypting `static`, and checks `m1` with that device's `psk`. But the device has not yet proved it holds the new session keys. **Key confirmation for the server is implicit until the first transport packet from the device decrypts under `tki`** | [WG] behaviour; the data phase is outside the paper's analysis. **BLOCKED B-08** |

**Consequences (B-08):**
- Until that first packet, the server keeps the new session in `PENDING` ([state-machine.md](state-machine.md)).
- **`PENDING` restricts transport data, not handshake messages.** Under the pending keys the server sends **no transport data**, forwards nothing, and does not report the device as authenticated (D-020, S-13).
- The server **still answers handshakes**: the RespHello that created the session has already been sent. A later valid InitHello from the same device (a retransmission after a lost RespHello, D4) gets a fresh RespHello and **replaces** the pending session (state-machine S1).
- Those answers are limited only by the normal InitHello checks and the pre-authentication budgets (B-06), never by the `PENDING` state.
- The device reports stage 2 ("peer authenticated") only after `empty` verifies.

## 8. Protocol identity, negotiation and downgrade

**Rule:** there is **no in-band negotiation** in SVP version 1.
- A *protocol identifier* names the version and the full parameter set (§4), and is hashed into `C1`/`H1` through `lbl1` and `lbl2`.
- This is the Noise approach: the protocol name initialises the symmetric state (Noise §5.2). WireGuard uses the same approach.
- If the two sides use different identifiers, they derive different keys. `static` then fails to decrypt on the server, the InitHello is dropped, and no shared key results.

**What this does and does not protect (qualified claim).**
- **Within the handshake:** an attacker cannot make an *enrolled* device and its server complete a handshake under a parameter set other than the one both have pinned, unless the attacker holds a party's private keys. This is a property of the draft design and is **BLOCKED B-07 / B-09** until G1. It is not a proven result.
- **It is only as strong as enrollment.** The identifier and the server's static keys that a device uses are whatever it pinned at enrollment (B-09). An attacker who could alter that data during enrollment could pin a weaker identifier or their own keys, and the handshake would then faithfully "protect" the wrong thing. Downgrade resistance therefore also needs all of the following (§10):
  - the invitation fingerprint covers the protocol identifier as well as the server's keys, so a changed identifier fails the fingerprint check (EN-6);
  - the enrollment channel itself accepts only the hybrid key exchange and has no fallback to a classical-only exchange or an older version;
  - the invitation is carried from the server to the device by a path the attacker cannot alter: shown on the server's own screen or administrator console and scanned by the user.
- **Not covered:** a compromised server or device, or an attacker who controls the invitation path. These are outside the claim ([threats.md](../spec/threats.md) ADV-05, ADV-07, ADV-08). A leaked invitation (ADV-06) is handled by administrator confirmation (EN-7), not by this mechanism.

| Item | Specification | Status |
| --- | --- | --- |
| Identifier contents | Protocol name, version, `DH`, `EPHEMERAL_KEM`, `STATIC_KEM`, `HASH`, `AEAD`. Also covered by the invitation fingerprint (§10.1) | **BLOCKED B-07** (exact strings) |
| Which identifier a peer uses | Fixed for each device at enrollment, as part of the pinned server record. A device never retries with another identifier on failure | **BLOCKED B-09** |
| Changing identifier (algorithm agility, S-05) | Only by an explicit, authenticated change: re-enrollment, or a later reviewed migration procedure. Never automatic fallback | **OPEN**, after G1 |
| Version mismatch shown to the user | Locally, as "handshake not answered", never "connected". The wire gives no error (§9) | [state-machine.md](state-machine.md) |

## 9. Processing rules

### 9.1 Device sending InitHello

1. Generate `sidi`, `(eci, Eci)` and `(epqi, Epqi)`; compute `ct1`/`shk1` with fresh randomness.
2. Run steps 1–5 of §6; build InitHello (§5.1).
3. Keep `(eci, epqi, C4, H5)` for this attempt only. Retransmission rules are in [state-machine.md](state-machine.md).

### 9.2 Server receiving InitHello

**Order of processing [HWG].** The server must identify the device before it can check `m1`. It therefore performs, in this order:
1. Length and `type` checks (M2-03).
2. **Validation of received keys [STD]:**
   - `Epqi` passes the FIPS 203 encapsulation-key input checks (§7.2) when `EPHEMERAL_KEM` is ML-KEM;
   - lengths match the parameter set exactly.
3. `DH(scr, Eci)`. Reject an all-zero result: RFC 7748 §6.1 permits this check, and B-10 asks the reviewer whether to require it.
4. Decrypt `ct1enc`; `shk1 = STATIC_KEM.Decaps(spqr, ct1)`.
5. Steps 1–3 of §6; `AEAD.Dec` of `static` → `HASH(Sci ‖ Spqi)`.
6. Look up the enrolled device by that hash. **Unknown or revoked → drop** (RV-1, RV-2).
7. Steps 4–5 of §6 using that device's keys and `psk`; decrypt `time`.
8. Verify `m1` with `HASH(lbl3 ‖ psk)`.
9. Check `time` (replay rule below).

**Cost before authentication.** Steps 2–5 cost a post-quantum decapsulation, X25519 and AEAD **before any authentication**. The paper notes this and did not analyse denial of service. **Under load, the cookie and budget rules of M2-03 must apply before step 3** (B-06, S-07, Q12).

**On success:** generate `sidr` and `(ecr, Ecr)`; compute `ct2`/`shk2` and `ct3`/`shk3` with fresh randomness; run steps 6–9; build RespHello; derive `(tki, tkr)`; enter `PENDING` (§7).

**Replay of InitHello.**
- The server keeps, per device, the greatest `timestamp` accepted, and drops any InitHello whose timestamp is not strictly greater [WG].
- In WireGuard this is a 12-byte TAI64N value.
- The paper notes that an attacker who controls the device's clock can block future handshakes, and did not analyse this.
- **BLOCKED B-11.** The reviewer should also consider LO-3: security must not depend on accurate client clocks. A clock set *backwards* only blocks that device's own handshakes, but the effect on availability must be documented.

### 9.3 Device receiving RespHello

1. Length/`type` checks; `sidi` must match the outstanding attempt.
2. Validate `ct2` length and `Ecr`.
3. Check `DH(eci, Ecr)` for an all-zero result (B-10).
4. Decrypt `ct3enc`; `shk2 = Decaps(epqi, ct2)`; `shk3 = Decaps(spqi, ct3)`.
5. Steps 6–9 of §6; `AEAD.Dec` of `empty`. Verify `m1`.
6. **Any failure → drop silently and keep waiting** until the attempt times out ([state-machine.md](state-machine.md)).
7. On success: derive `(tki, tkr)`, erase `eci` and `epqi`, and enter `ESTABLISHED`.

### 9.4 Errors on the wire

- **No error messages are sent** in response to a bad handshake message: invalid messages are dropped silently [WG]. This avoids an oracle and amplification.
- Each side records a **local reason code** for diagnostics, without secrets (PR-2), for example `unknown_device`, `bad_mac`, `stale_timestamp`, `decap_failed` or `rate_limited`.
- The list of codes is in [state-machine.md](state-machine.md) §6.

### 9.5 Erasure

- Ephemeral private keys, KEM shared secrets, `κ` values and intermediate `C` values are erased as soon as they are no longer needed. Erasure is best effort (KS-5, S-03).
- The exact lifetimes are in [state-machine.md](state-machine.md).

## 10. Enrollment (how the keys and PSK get there)

**Status: requirements only. The construction is BLOCKED B-12 (owner question M2-Q7).**

The Hybrid-WireGuard paper assumes that the static keys and `psk` are already in place. Enrollment must deliver:

| Direction | Content | Protection needed |
| --- | --- | --- |
| Server → device | `Scr`, `Spqr` (up to about 0.5 MB with Classic McEliece), the protocol identifier (§8), and `psk` (if M2-Q6 (a)) | Integrity, checked against the invitation fingerprint (EN-6). **Confidentiality for `psk` that is itself post-quantum (hybrid)**: a `psk` recorded today and broken later would remove the properties that rely on it |
| Device → server | Proof of the one-time invitation secret (EN-1, EN-4, EN-5), `Sci`, `Spqi`, device name | Bound to the same authenticated channel (channel binding), so a stolen proof cannot be reused elsewhere |
| Both | Short confirmation code (EN-7), derived from the channel so both sides show the same value | Integrity |

**Options for the construction (none chosen):**

| Option | What it is | For | Against |
| --- | --- | --- | --- |
| **(a) TLS 1.3 with hybrid key exchange** | TLS 1.3 using X25519MLKEM768 (RFC 10024), the server authenticated by a raw public key (RFC 7250) whose fingerprint is in the invitation. The invitation proof and confirmation code are bound with a TLS exporter (RFC 5705 / RFC 9266 style) | Every part is a published standard with analysis; mature Rust TLS | Needs a server **signing** key: Ed25519, or ML-DSA-65 + Ed25519 (candidate D-012). Needs a transport (TCP, or QUIC on the same UDP port), which affects Q12 and M2-03. The glue is still **BLOCKED** |
| **(b) A password-authenticated key exchange (PAKE)** keyed by the invitation secret | OPAQUE (RFC 9807) or CPace (draft) | Standard designs that turn a short secret into a key | Classical only; no standard post-quantum PAKE. `psk` confidentiality would not be post-quantum |
| **(c) Out-of-band file copy** | The administrator copies keys between machines | No new protocol | Poor usability, against the M1 enrollment flow; still needs integrity checks |

**Recommendation for the reviewer to assess:** (a). Its pieces are standards, it gives hybrid post-quantum confidentiality for the `psk`, and authentication at enrollment time only has to resist attackers who exist *now*. The full design is written in M2-03/M2-04 only if the reviewer agrees with the direction.

### 10.1 How the TLS signing identity is bound to the SVP identity and the invitation (option (a), draft)

**Problem.**
- Under option (a), the key that authenticates the server in TLS is a **signing** key (`Ksig`), which is not used by the SVP handshake.
- The device must be sure that the `Ksig` it talked to, and the SVP keys (`Scr`, `Spqr`) and protocol identifier it receives, all belong to the **same** server named in the invitation.
- Otherwise an attacker could pair a genuine channel with substituted SVP keys, or the reverse.

**Binding, step by step (BLOCKED B-02, B-12):**

| Step | What happens | What it binds |
| --- | --- | --- |
| 1. One server identity | The server identity is the set {`Ksig`, `Scr`, `Spqr`, protocol identifier}. All are generated together on the server (ID-1), stored together (KS-1), backed up together (Q8) and replaced together (RV-5). `Ksig` is used only for enrollment, never in SVP handshakes | The keys are one identity, not separate ones |
| 2. Invitation fingerprint | `fingerprint = HASH_fp(encode(Ksig) ‖ encode(Scr) ‖ encode(Spqr) ‖ encode(identifier))`. Hash, unambiguous encoding and truncation are **B-02** | The invitation names every part, including the identifier (§8) |
| 3. TLS authentication | TLS 1.3 with the server authenticated by `Ksig` as a raw public key (RFC 7250); only the hybrid key exchange is offered and accepted. TLS's own handshake signature proves possession of `Ksig` over the TLS transcript | The channel ends at the holder of `Ksig` |
| 4. Keys sent inside the channel | The server sends `Scr`, `Spqr` and the identifier inside that TLS connection | They come from the `Ksig` holder, protected by TLS |
| 5. Device check | The device recomputes the fingerprint from the `Ksig` **actually used in the TLS handshake** plus the received `Scr`, `Spqr` and identifier, and compares it with the invitation. Any mismatch: stop, report possible interception, store nothing (EN-6) | The channel key and the SVP keys match the invitation as one set |
| 6. Invitation proof bound to the channel | The device proves the one-time secret with a value derived from a **TLS exporter** of this connection (channel binding, RFC 9266 style), not the bare secret | A proof captured on another connection, or by a different server, is useless |
| 7. Confirmation code | The EN-7 short code is also derived from the TLS exporter, so both sides show the same code only on the same channel | The administrator's approval applies to this channel and this device |
| 8. After enrollment | The device pins `Scr`, `Spqr`, the identifier and the fingerprint. `Ksig` is kept only to recognise the server at a later re-enrollment | The SVP handshake relies on the pinned set (§2) |

**Open for the reviewer:**
- the exporter labels and the derivation of the proof and code (B-12);
- whether `Ksig` should be Ed25519 or ML-DSA-65 + Ed25519 (candidate D-012);
- whether the server should *also* sign the SVP keys with `Ksig` (redundant with steps 2 and 5, but it would allow verification outside the channel).

## 11. Identity replacement and revocation

| Case | Rule | Source / status |
| --- | --- | --- |
| Device revoked | The server removes the record. Step 6 of §9.2 then drops every InitHello; existing sessions are closed and their keys erased (RV-1) | M1 |
| Device key replaced (ID-5) | New keys are enrolled as a new device record; the old record is revoked. Overlap rules are **BLOCKED B-13** | M1 + B-13 |
| Server key replaced or lost | Devices refuse the new identity (RV-5) until re-enrollment or restore from the Q8 backup | M1 |
| Session lifetime | Sessions re-authenticate within a bounded time (RV-6): the timers in [state-machine.md](state-machine.md) | [WG] values, B-14 |

## 12. Register of BLOCKED items (reviewer: read this first)

| ID | Item | Why it is not covered by an existing analysis |
| --- | --- | --- |
| B-00 | Our **transcription** of the analysed design (§5, §6, §9.2 order) | Must be checked against the paper (Tables 6 and 7, Appendix A); the extracted text was hard to read in places |
| B-01 | Device-only initiation | Septuvo restriction; changes timer and collision behaviour |
| B-02 | Fingerprint construction (hash, encoding, truncation), including what it covers (§10.1) | New |
| B-03 | `EPHEMERAL_KEM`, `STATIC_KEM` parameter sets | Any choice other than ML-KEM-512 + Classic McEliece level 3 departs from the analysed parameters (M2-Q3) |
| B-04 | Per-device `psk`, or all-zero `psk` | Analysed properties assume a secret `psk`; how Septuvo creates it is new (M2-Q6) |
| B-05 | `HASH`, `MAC`, `KDF`, `AEAD` and HKDF conventions | Keeping the paper's choices is low risk; any change is a substitution (M2-Q5) |
| B-06 | Cookie / `m2` mechanism and pre-authentication budgets | Not analysed in the paper; Septuvo's own design in M2-03 |
| B-07 | Label strings and protocol-identifier format | New strings |
| B-08 | Server-side key confirmation by first transport packet; `PENDING` rules | Data phase outside the paper's analysis |
| B-09 | Identifier fixed at enrollment; no retry with another | Septuvo policy |
| B-10 | Rejecting all-zero X25519 results | RFC 7748 makes it optional; the decision is ours |
| B-11 | Timestamp format and replay rule | Not analysed in the paper; interacts with LO-3 |
| B-12 | Enrollment construction, including the binding of the TLS signing key to the SVP identity (§10.1) | New composition |
| B-13 | Key-replacement overlap | New |
| B-14 | Timer and message-count values | Taken from WireGuard; not analysed for this protocol or for AES-GCM limits (M2-03) |

## 13. Owner questions (M2-02)

| Q | Question | Recommendation | Owner response (2026-10-15) |
| --- | --- | --- | --- |
| **M2-Q5** | Inside the handshake, keep the analysed symmetric primitives (BLAKE2s, HKDF-HMAC-BLAKE2s, ChaCha20-Poly1305), or switch to SHA-256-based HKDF to match D-024's usual form? | **Keep the analysed ones.** Fewer substitutions for the reviewer to re-check. The data-phase cipher is decided separately (D-010) | Accepted as the **preferred candidate**; provisional baseline, subject to G1 |
| **M2-Q6** | PSK: a random per-device secret created at enrollment, or a fixed all-zero value? | **Per-device secret.** It keeps the analysed design's assumption, but it requires enrollment to protect it with post-quantum confidentiality (§10) | Acceptable direction for drafting; subject to G1 |
| **M2-Q7** | Enrollment construction: TLS 1.3 with hybrid key exchange, a PAKE, or manual copying? | **TLS 1.3 with hybrid key exchange (RFC 10024)**, to be confirmed by the reviewer. It adds a server signing key, so it touches D-012 | Acceptable direction for drafting; subject to G1 |
| M2-Q3 | Static and ephemeral KEM parameter sets | Still **open**; the reviewer is asked ([candidate-analysis.md](candidate-analysis.md) §5) | Open |

None of these is a final decision. Each stays BLOCKED until G1 and the owner's decision after it.

## 14. Coverage of the review checklist

| Checklist item ([PROTOCOL_REVIEW_CHECKLIST.md](../PROTOCOL_REVIEW_CHECKLIST.md)) | Where | Status |
| --- | --- | --- |
| P-03 Identity and enrollment | §2, §3, §10, §11 | Draft; enrollment **BLOCKED B-12** |
| P-04 Hybrid handshake | §5, §6, §9 | Draft; parameters **BLOCKED B-03** |
| P-05 Transcript and KDF | §6, §8 | Draft; labels and HKDF conventions **BLOCKED B-05, B-07** |
| P-06 Cipher suites, negotiation | §4, §8 | Draft; no in-band negotiation |
| P-12 Error semantics | §9.4, [state-machine.md](state-machine.md) | Draft |
| P-02, P-07 to P-11 | M2-03 | Not started |
| P-13, P-14 | M2-04, M2-05 | Not started |

## Sources

- Lafourcade et al., "A Tale of Two Worlds, a Formal Story of WireGuard Hybridization", USENIX Security 2025: <https://www.usenix.org/conference/usenixsecurity25/presentation/lafourcade>
  - Full paper: <https://www.usenix.org/system/files/usenixsecurity25-lafourcade.pdf>
  - Appendix: <https://www.usenix.org/system/files/usenixsecurity25-appendix-lafourcade.pdf>
  - ePrint 2025/1179: <https://eprint.iacr.org/2025/1179>
- WireGuard whitepaper (Donenfeld): <https://www.wireguard.com/papers/wireguard.pdf>
- Noise Protocol Framework, revision 34: <https://noiseprotocol.org/noise.html>
- FIPS 203 (ML-KEM): <https://csrc.nist.gov/pubs/fips/203/final>
- RFC 7748 (X25519): <https://www.rfc-editor.org/rfc/rfc7748>
- RFC 5869 (HKDF): <https://www.rfc-editor.org/rfc/rfc5869>
- RFC 10024 (hybrid key exchange in TLS 1.3): <https://www.rfc-editor.org/rfc/rfc10024>
- RFC 7250 (raw public keys in TLS): <https://www.rfc-editor.org/rfc/rfc7250>
- RFC 9807 (OPAQUE): <https://www.rfc-editor.org/rfc/rfc9807>
