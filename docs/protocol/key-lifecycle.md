# SVP — key lifecycle, rotation and crash recovery (M2-03, draft)

**Status:** draft for owner review and the G1 specialist review. **Not approved; no code.**
- It lists every piece of secret and long-term key material in SVP: how it is created, where it lives, how long it lasts, how it is replaced and how it is erased.
- Companion to [handshake.md](handshake.md), [state-machine.md](state-machine.md) and [wire-format.md](wire-format.md). The marks and the BLOCKED register continue from there.

**License:** Apache-2.0 (open SVP specification, D-033).

## 1. Inventory of key material

| Material | Created by / when | Stored where | Lifetime | Erased when | Source / status |
| --- | --- | --- | --- | --- | --- |
| Device static keys `sci`, `spqi` | Device, before enrollment (EN-8), from the OS random-number generator | Device key store (KS-1, KS-2); never leaves the device (ID-2) | Until the device is replaced or revoked | Device reset or uninstall (best effort, KS-5) | M1 |
| Server static keys `scr`, `spqr` | Server, at setup | Server key store (KS-1); optional encrypted offline backup (Q8) | Until replaced (RV-5) | Replacement | M1 |
| Server enrollment signing key `Ksig` | Server, at setup, together with the static keys | As above | Same as the server identity ([handshake.md](handshake.md) §10.1) | As above | M2-Q7 direction; **B-12** |
| Per-device `psk` | Server, at enrollment; sent over the enrollment channel | Both sides, as secret key material (KS-1) | Until the device is re-enrolled or revoked | Revocation (server); reset (device) | M2-Q6 direction; **B-04** |
| Invitation secret | Server, per invitation | Server stores only a verifier (EN-4); the device has it briefly | 24 h default, 7 days at most (Q7); single use (EN-1) | Use, expiry or cancellation | M1 |
| Ephemeral keys `eci`, `epqi`, `ecr` and KEM randomness | Per handshake attempt | Memory only | One attempt | As soon as the step that uses them completes ([handshake.md](handshake.md) §9.5) | [HWG] |
| Handshake intermediates (`C`, `κ`, `H`, `shk1..3`, DH values) | During a handshake | Memory only | One handshake | As soon as no longer needed | [HWG] |
| Session keys `tki`, `tkr` | End of a handshake | Memory only, never written to disk or swap where the platform allows | Until the limits of §3–§4 | Expiry, replacement, revocation or disconnect | [HWG]; limits **B-14, B-17** |
| Cookie secret `Rm` | Server, at random | Memory only | 120 s | Replacement | [WG]; **B-06** |
| Cookie (device side) | Received in a CookieReply | Memory only | 120 s | Expiry | [WG] |
| Greatest accepted timestamp per device | Server | **Persisted** (§6) | While the device is enrolled | Revocation | [WG]; **B-11** |

**General rules:**
- Session keys, ephemeral keys and intermediates are **never persisted**. That is what makes crash recovery safe (§6).
- Logs and diagnostics never contain any of the material above (PR-2).
- Erasure from memory is best effort and its limits are documented (KS-5, S-03).

## 2. Epochs and keypair slots

**Each completed handshake starts a new epoch** with fresh `tki`/`tkr`. Keys are never derived from earlier session keys.
- Forward secrecy comes from the ephemeral exchange ([handshake.md](handshake.md) §6, steps 7–8).
- No ratchet is added. A ratchet would be a new construction.

| Slot | Device | Server |
| --- | --- | --- |
| `current` | Sends with it from D2/D8 onward | Sends with it only after S3 (confirmation) |
| `previous` | Receive-only, until the server's first packet under the new keys arrives or `REJECT_AFTER_TIME` | Receive-only until `REJECT_AFTER_TIME` |
| `pending` / in-flight | The handshake attempt in progress | `PENDING` keys awaiting confirmation; **no transport data** sent under them ([handshake.md](handshake.md) §7) |

- At most **three** sets of session keys exist per device at any moment.
- A slot that is replaced or expires is erased at once ([state-machine.md](state-machine.md) S3, S5, D10).

## 3. Rotation triggers (time and use)

Reference values are in [state-machine.md](state-machine.md) §1 (**B-14**).

| Trigger | Who acts | Effect |
| --- | --- | --- |
| Session age ≥ `REKEY_AFTER_TIME` (120 s) | Device | New handshake (D6) |
| Packets sent with one key ≥ `REKEY_AFTER_MESSAGES` | Device | New handshake (D6) |
| Device receiving on a session that is about to expire | Device | New handshake (D7), because the server never initiates (B-01) |
| Session age ≥ `REJECT_AFTER_TIME` (180 s), or the per-key limit of §4 reached | Both | Keys erased and never used again |
| Revocation | Server | All keys of that device erased at once (RV-1) |

The bounded session lifetime (`REJECT_AFTER_TIME`) is also what meets RV-6: a device re-authenticates at least every few minutes and cannot keep access by caching anything.

## 4. Per-key usage limits by data cipher (B-17)

The data cipher is still a candidate (D-010), and its usage limits **decide** `REKEY_AFTER_MESSAGES` and the reject limit. The values must be fixed with the reviewer from the published analyses, for the actual maximum packet size of [wire-format.md](wire-format.md) §5:
- the CFRG "Usage Limits on AEAD Algorithms" draft;
- the analysis used for QUIC (RFC 9001 §6.6).

| `DATA_AEAD` | Confidentiality limit (packets per key) | Integrity limit (forgeries tried per key) | Consequence for SVP |
| --- | --- | --- | --- |
| ChaCha20-Poly1305 | Not a practical limit at SVP packet counts (the QUIC analysis treats it as above 2^62) | About 2^36 failed decryptions, per the QUIC analysis | WireGuard's 2^60 / 2^64 − 2^13 − 1 are usable; count failed decryptions per key |
| AES-256-GCM | **Much lower**: on the order of 2^23–2^25 packets per key in the QUIC analysis, depending on assumed packet size | On the order of 2^52–2^54 | `REKEY_AFTER_MESSAGES` must be set **well below** the confidentiality limit. At 10 Gbit/s with full-size packets, about 2^23 packets is roughly ten seconds of traffic, so packet count, not time, would trigger most rekeys on fast links |

**Rules for both ciphers:**
- **Count and enforce per key:** packets sent (sender) and failed decryptions (receiver). On reaching the integrity limit, the receiver stops accepting packets under that key and a new handshake is started.
- **The figures above are planning ranges, not specification values.** Exact numbers are BLOCKED B-17 until the reviewer confirms them for SVP's packet sizes.
- Rekeying more often costs handshakes, so the **server's handshake budget** ([wire-format.md](wire-format.md) §6.3) must allow the measured rekey rate of busy devices. This is a G2 measurement.

## 5. Long-term identity rotation and loss

| Case | Procedure | Status |
| --- | --- | --- |
| **Device key replacement** (ID-5) | Enroll the new keys as a new device record using a new invitation; after the first successful handshake with the new keys, the administrator (or an automatic rule) revokes the old record. Until then, both records exist with the **same scope** | Overlap rule **BLOCKED B-13** |
| **Device lost or compromised** (RV-3, RV-4) | Revoke; enroll the replacement. Keys are never recovered | M1 |
| **`psk` replacement** | Only by re-enrollment in version 1. No in-band PSK change | **B-04** |
| **Server key replacement** (planned) | **Not supported in-band in version 1.** Devices refuse a changed server identity (RV-5), so a planned change means re-enrolling every device, or restoring the old identity from the Q8 backup. An authenticated migration procedure is **OPEN** for after G1 | M1 + OPEN |
| **Server key lost** | Restore from the Q8 encrypted backup, or re-enroll every device | M1 |
| **Change of protocol identifier** (algorithm migration) | Re-enrollment; never automatic fallback ([handshake.md](handshake.md) §8) | OPEN, after G1 |

## 6. Crash, restart and recovery

| Event | Behaviour | Why it is safe | Status |
| --- | --- | --- | --- |
| **Device crash or reboot** | All session state is lost. The kill switch stays in place (N1-03), the device starts a new handshake (D1), and status shows "not connected" until all four stages pass | Session keys are never persisted, and a new handshake gives **new keys**, so a packet counter restarting at 0 can never repeat a nonce under an old key | [WG] |
| **Server crash or restart** | All slots start `NONE` (S8). Devices notice and re-handshake (D7, D10) | As above. No nonce reuse is possible | [WG] |
| **Replay of an old InitHello after a server restart** | The server persists each device's greatest accepted timestamp and checks it after restart. If the persisted value were lost, an attacker could replay a recorded InitHello. That would only create a `PENDING` session the attacker cannot use, which then replaces the device's own pending handshake, which is a disruption. It would not break confidentiality | Replay gives no keys | **B-11:** persist before replying, or accept the disruption; the reviewer decides |
| **Clock set backwards on a device** | Its InitHellos are refused until its clock passes the last accepted value. Other devices are unaffected. This is an availability problem only (LO-3) | No security effect | **B-11** |
| **Storage corruption of authorization data** | The server refuses all devices (RV-2, S9) | Fail closed | M1 |
| **Partial write during enrollment** | Enrollment completes only when both the device record and the consumption of the invitation are stored atomically (EN-1); otherwise neither | Single use is preserved | **B-12** |

## 7. Erasure (zeroization)

- Every buffer that held material from §1 is overwritten before release, using a method the compiler cannot remove.
- Copies made by the operating system (swap, crash dumps, hibernation files) are reduced where the platform allows: locked memory, crash dumps kept local only (KS-5, PR-6). What each platform actually achieves is recorded in [platform-feasibility.md](../spec/platform-feasibility.md) and verified in M7.
- **No claim of complete erasure is made** (S-03).

## 8. Coverage of the review checklist

| Item | Where | Status |
| --- | --- | --- |
| P-09 Rotation and lifetime | §2–§5 | Draft; **B-13, B-14, B-17** |
| P-07 Packet numbering (crash and failover) | §6 | Draft |
| P-03 Identity recovery and revocation | §5 | Draft; server migration OPEN |

## Sources

- WireGuard whitepaper §6 (timers and key rotation): <https://www.wireguard.com/papers/wireguard.pdf>
- RFC 9001 §6.6 (AEAD usage limits in QUIC): <https://www.rfc-editor.org/rfc/rfc9001>
- CFRG draft "Usage Limits on AEAD Algorithms": <https://datatracker.ietf.org/doc/draft-irtf-cfrg-aead-limits/>
- NIST SP 800-38D (GCM): <https://csrc.nist.gov/pubs/sp/800/38/d/final>
