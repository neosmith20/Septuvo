# SVP — handshake and session state machine (M2-02, draft)

**Status:** draft for owner review and the G1 specialist review. Companion to [handshake.md](handshake.md); it uses that document's notation and BLOCKED register.
- Every state and transition is listed explicitly. Anything not listed is **not allowed**: the event is dropped and a local reason code recorded (§6).
- Timer values are **reference values from WireGuard** (whitepaper §6). They are not analysed for SVP, so all are **BLOCKED B-14**.
- Data-phase details (packet numbers, replay windows, epochs, AEAD limits) are M2-03.

**License:** Apache-2.0 (open SVP specification, D-033).

## 1. Timers and limits (reference values, BLOCKED B-14)

| Name | Reference value [WG] | Meaning |
| --- | --- | --- |
| `REKEY_AFTER_TIME` | 120 s | The device starts a new handshake when its current session is this old |
| `REJECT_AFTER_TIME` | 180 s | Neither side uses session keys older than this; they are erased |
| `REKEY_ATTEMPT_TIME` | 90 s | The device stops retrying a handshake after this long |
| `REKEY_TIMEOUT` | 5 s, plus random jitter (WireGuard: 0–333 ms) | Wait before retransmitting InitHello; at most one InitHello per period |
| `KEEPALIVE_TIMEOUT` | 10 s | Used in the receive-side rekey rule (§3) |
| `REKEY_AFTER_MESSAGES` | 2^60 | Start a new handshake after this many packets with one key |
| `REJECT_AFTER_MESSAGES` | 2^64 − 2^13 − 1 | Never use a key for more packets than this |

**Not final.** The message limits come from WireGuard's ChaCha20-Poly1305 nonce space. If the data phase uses AES-256-GCM (candidate D-010), M2-03 must set limits from that cipher's own usage bounds.

## 2. Device (initiator) states

| State | Meaning | Kill switch | Status stage shown (D-020) |
| --- | --- | --- | --- |
| `STOPPED` | Not connecting; user turned it off | As configured by policy (blocking by default for protected traffic) | Not connected |
| `HANDSHAKE_SENT` | InitHello sent, waiting for RespHello; no usable session | **Blocking** | Adapter only |
| `ESTABLISHED` | Session keys from a verified RespHello; may send data | Blocking except through the tunnel | Peer authenticated (stage 2). Stages 3–4 come from independent probes |
| `REKEYING` | `ESTABLISHED`, plus a new handshake in flight. The current keys are still used until the new ones are confirmed | Same as `ESTABLISHED` | As `ESTABLISHED` |
| `FAILED` | Retries exhausted, or a fatal local error | **Blocking** | Not connected, with the reason |

### Device transitions

| # | From | Event | Guard | Action | To |
| --- | --- | --- | --- | --- | --- |
| D1 | `STOPPED` | User connects | Enrolled; server record present | Build and send InitHello (handshake.md §9.1); start `REKEY_TIMEOUT` and `REKEY_ATTEMPT_TIME` | `HANDSHAKE_SENT` |
| D2 | `HANDSHAKE_SENT` | Valid RespHello | All checks of handshake.md §9.3 pass | Derive `tki`/`tkr`; erase the attempt's ephemeral secrets; start the session age clock; **send the first transport packet (a keepalive if there is no data) to confirm the keys to the server** | `ESTABLISHED` |
| D3 | `HANDSHAKE_SENT` | Invalid RespHello | Any check fails | Drop; record a reason code; keep waiting | `HANDSHAKE_SENT` |
| D4 | `HANDSHAKE_SENT` | `REKEY_TIMEOUT` expires | `REKEY_ATTEMPT_TIME` not reached | New InitHello with **new** ephemeral keys and a new `sidi`. A retransmission is never a byte-for-byte resend | `HANDSHAKE_SENT` |
| D5 | `HANDSHAKE_SENT` | `REKEY_ATTEMPT_TIME` expires | — | Erase the attempt's secrets; report "server not answering" | `FAILED` |
| D6 | `ESTABLISHED` | Session age ≥ `REKEY_AFTER_TIME`, or packets ≥ `REKEY_AFTER_MESSAGES` | No handshake already in flight | Send InitHello | `REKEYING` |
| D7 | `ESTABLISHED` | Packet received and session age ≥ `REJECT_AFTER_TIME − KEEPALIVE_TIMEOUT − REKEY_TIMEOUT` | No handshake in flight | Send InitHello. This is WireGuard's receive-side rule; it is needed because the server never initiates (B-01) | `REKEYING` |
| D8 | `REKEYING` | Valid RespHello | As D2 | New keys become current; old keys become *previous* until `REJECT_AFTER_TIME` or until the server's first packet under the new keys arrives | `ESTABLISHED` |
| D9 | `REKEYING` | `REKEY_TIMEOUT` / `REKEY_ATTEMPT_TIME` | As D4 / D5 | Retry as D4. On attempt time-out, keep the current keys **only** while their age < `REJECT_AFTER_TIME` | `REKEYING` / `ESTABLISHED` |
| D10 | `ESTABLISHED`, `REKEYING` | Current keys reach `REJECT_AFTER_TIME` or `REJECT_AFTER_MESSAGES` | — | Erase the keys; stop sending under them; if no new session exists, start a handshake as D1 | `HANDSHAKE_SENT` |
| D11 | Any | User disconnects | — | Erase all session and attempt secrets | `STOPPED` |
| D12 | `FAILED` | User retries, or the network changes | — | As D1 | `HANDSHAKE_SENT` |
| D13 | Any | Pinned server record missing or unreadable | — | Erase secrets; report "enrollment data damaged". Never connect without a pinned identity | `FAILED` |

**Fail-closed invariant (S-08, NP rules):** in no device state does a handshake failure, time-out or key expiry open a path outside the tunnel for protected traffic. The kill switch ([N1-03 lab](../build/lab.md)) stays in force.

## 3. Server (responder) per-device states

Each enrolled device has at most **one** session in each slot:
- `current`: in use;
- `previous`: kept only to receive late packets;
- `pending`: new keys not yet confirmed by the device.

| State (of a slot) | Meaning |
| --- | --- |
| `NONE` | No keys |
| `PENDING` | RespHello sent; keys derived, **not yet confirmed** (handshake.md §7, B-08). **No transport data** is sent or forwarded under these keys, and the device is **not** reported as authenticated. Handshake processing is **not** blocked: a new valid InitHello is answered with a new RespHello and replaces this slot (S1) |
| `CURRENT` | Confirmed by the first valid transport packet under `tki` |
| `PREVIOUS` | The former `CURRENT` after a rekey; receive-only until it expires |

### Server transitions

| # | Event | Guard | Action | Result |
| --- | --- | --- | --- | --- |
| S1 | InitHello | Passes handshake.md §9.2 (device enrolled, not revoked, `m1` valid, timestamp newer) **and** the pre-authentication budget of M2-03 allows the work (B-06) | Build RespHello; derive keys; **replace** any existing `pending` (erasing its secrets); record the new greatest timestamp | `pending` := new |
| S2 | InitHello | Any check fails | Drop silently; record a reason code; **no reply** | No change |
| S3 | Transport packet decrypting under `pending.tki` | Replay checks of M2-03 | `previous` := `current`; `current` := `pending`; `pending` := `NONE`. Erase the old `previous`. Mark the device **authenticated** for status and policy | Session confirmed |
| S4 | Transport packet under `current` or `previous` | Replay checks | Process normally (authorization per packet, NP-G1) | No change |
| S5 | Slot age ≥ `REJECT_AFTER_TIME`, or packet count ≥ `REJECT_AFTER_MESSAGES` | — | Erase the slot's keys | Slot := `NONE` |
| S6 | `pending` older than `REKEY_TIMEOUT × 3` without confirmation | — | Erase `pending` | `pending` := `NONE`. **Value BLOCKED B-14**: WireGuard keeps an unconfirmed session until it is replaced or expires; the bound here limits memory |
| S7 | Device revoked by the administrator | — | Erase all three slots at once; drop the device's packets (RV-1) | All `NONE`; later InitHellos fail at S2 |
| S8 | Server restart or crash | — | All slots start `NONE`; nothing is restored from disk. The timestamp table is persisted so that old InitHellos cannot be replayed after a restart (**BLOCKED B-11**) | Devices re-handshake (D7, D10) |
| S9 | Authorization data unreadable | — | Refuse all devices (RV-2) | All `NONE` |

**Never** in any server state:
- send transport data to a device under keys that have not reached S3. Handshake responses (RespHello, and the cookie replies of M2-03) are not transport data and are allowed in every state, subject to S1/S2 and the budgets;
- treat a `PENDING` session as authenticated;
- forward packets whose destination the device's scope does not allow (S-09).

## 4. Status stages (D-020) and what the states prove

| Stage | Proven by | Not proven by |
| --- | --- | --- |
| 1. Adapter active | OS reports the tunnel interface up | — |
| 2. Peer authenticated | **Device:** D2/D8 (`empty` verified). **Server:** S3 | An interface being up; an InitHello merely sent or received |
| 3. Reachability | An independent probe through the tunnel succeeds (N1-04 checks) | Stage 2 alone |
| 4. DNS | The tunnel's DNS resolver answers through the tunnel | Stages 1–3 alone |

`septuvo status` reports "connected" only when stages 1–4 all pass, in order (N1-04).

## 5. Concurrency and collisions

- Only devices initiate (B-01), so the WireGuard case of both sides initiating at once does not occur.
- A device has at most **one** handshake in flight. A new attempt (D4) replaces the old one, whose secrets are erased. A late RespHello for a replaced attempt fails the `sidi` match and is dropped (D3).
- The server keeps at most one `pending` per device (S1), so repeated InitHellos cannot grow its memory per device. Global limits across devices and unknown sources are M2-03 (B-06).

## 6. Local reason codes (diagnostics only, never sent on the wire)

| Code | Where | Meaning |
| --- | --- | --- |
| `malformed` | Both | Wrong length or type |
| `bad_key` | Both | Received public key or ciphertext fails validation, or a DH result is zero (B-10) |
| `decap_failed` | Both | KEM decapsulation or `SE` step failed |
| `unknown_device` | Server | The `static` hash matches no enrolled device |
| `revoked` | Server | The device is revoked |
| `bad_mac` | Both | `m1` or AEAD verification failed |
| `stale_timestamp` | Server | Timestamp not newer than the last accepted (B-11) |
| `rate_limited` | Server | Pre-authentication budget exhausted (M2-03) |
| `no_answer` | Device | `REKEY_ATTEMPT_TIME` expired (D5) |
| `pin_missing` | Device | D13 |

Reason codes contain no key material, no invitation secrets and no packet contents (PR-2). Under load they are aggregated rather than logged one per packet, so logging cannot itself become a denial-of-service path (M2-03).

## 7. Open items handed to M2-03

- Byte layouts, `type` values, the session-ID uniqueness rule.
- Cookie / `m2` mechanism, anti-amplification and pre-authentication budgets (B-06).
- Packet numbers, replay windows, `previous`/`current` switch-over rules on the device side, and AEAD usage limits for the chosen data cipher (B-14).
- Roaming (address changes) and crash recovery for devices.
- Fragmentation, if the chosen parameter sets need it ([candidate-analysis.md](candidate-analysis.md) §5).

## Sources

- WireGuard whitepaper, §5–§6 (messages, timers, rekeying): <https://www.wireguard.com/papers/wireguard.pdf>
- Lafourcade et al., USENIX Security 2025 (see [handshake.md](handshake.md) for full references)
