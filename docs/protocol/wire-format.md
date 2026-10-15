# SVP — wire format, replay and denial-of-service handling (M2-03, draft)

**Status:** draft for owner review and the G1 specialist review. **Not approved, not an implementation guide, and no code exists** (D-021, D-029).
- It gives byte layouts for the messages of [handshake.md](handshake.md), the data-phase packet, replay protection, MTU and fragmentation, roaming, and Septuvo's own pre-authentication and anti-amplification design.
- Key lifetimes, rotation and crash recovery are in [key-lifecycle.md](key-lifecycle.md).
- Marks **[HWG]**, **[WG]**, **[STD]** and **BLOCKED B-nn** mean the same as in [handshake.md](handshake.md) §1. The BLOCKED register continues from there (§10).

**License:** Apache-2.0 (open SVP specification, D-033).

> **Denial-of-service protection here is Septuvo's own design.** The Hybrid-WireGuard analysis does not cover it ([candidate-analysis.md](candidate-analysis.md) §6). The cookie mechanism below follows WireGuard's published design, but WireGuard's cookie is not formally analysed either. Everything in §6 is **BLOCKED** until G1.

## 1. General encoding rules

| Rule | Specification | Source / status |
| --- | --- | --- |
| Transport | One SVP message per UDP datagram. No message spans datagrams, except under §5.3 | [WG] |
| Integers | Unsigned, **little-endian**, fixed width | [WG]; **B-15** |
| Fixed sizes | Every handshake message has **one exact size** for a given protocol identifier. A datagram whose length differs from the exact size for its `type` is dropped before any cryptographic work | **B-15** |
| Reserved bytes | Must be zero when sent. A non-zero value means drop | [WG] |
| Unknown `type` or `version` | Drop silently; reason code `malformed` | [state-machine.md](state-machine.md) §6 |
| Allocation | No allocation sized by a value read from the packet. All buffers are sized by `type` and the protocol identifier | S-07 |
| Order of checks | Length → `type`/`version`/reserved → under-load rule (§6) → cryptography | S-07 |

### Common header (4 bytes)

| Offset | Size | Field | Value |
| --- | --- | --- | --- |
| 0 | 1 | `type` | 0x01 InitHello, 0x02 RespHello, 0x03 CookieReply, 0x04 Transport. All others reserved |
| 1 | 1 | `version` | 0x01 for this draft |
| 2 | 2 | `reserved` | 0x0000 |

**Status of the header: BLOCKED B-15.**
- WireGuard uses `type` (1 byte) followed by 3 reserved bytes. The Hybrid-WireGuard paper describes a 4-byte `type`.
- Septuvo uses one of those bytes for `version`. The `version` byte is only a **sorting hint**. It is authenticated: by `m1` in handshake messages, which cover `inner`, and by associated data in transport packets (§4).
- The actual protocol identity is the identifier hashed into the key schedule ([handshake.md](handshake.md) §8).
- Every defined `type` has bit 0x40 clear. The QUIC header always sets that bit (RFC 9000 §17), so SVP and a QUIC-based enrollment service (handshake.md §10, option (a)) *could* share one UDP port (RFC 9443 style multiplexing). **Not decided**; this note only keeps the option open.

## 2. Handshake messages

Field contents are defined in [handshake.md](handshake.md) §5. Sizes use these parameter-dependent values:
- `EK_e`: the ephemeral KEM encapsulation key size;
- `CT_e`: the ephemeral KEM ciphertext size;
- `CT_s`: the static KEM ciphertext size.

AEAD tags are 16 bytes.

### 2.1 InitHello (type 0x01)

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | Header |
| 4 | 4 | `sidi` |
| 8 | 32 | `Eci` |
| 40 | `EK_e` | `Epqi` |
| 40 + `EK_e` | `CT_s` | `ct1enc` |
| … | 32 + 16 | `static`: AEAD of `HASH(Sci ‖ Spqi)`, which is 32 bytes for a 32-byte `HASH` |
| … | 12 + 16 | `time`: AEAD of a 12-byte timestamp (TAI64N in WireGuard; **B-11**) |
| … | 16 | `m1` |
| … | 16 | `m2` |

**Size:** `148 + EK_e + CT_s` bytes of UDP payload. `inner` is everything before `m1`.

### 2.2 RespHello (type 0x02)

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | Header |
| 4 | 4 | `sidr` |
| 8 | 4 | `sidi` (echoed) |
| 12 | 32 | `Ecr` |
| 44 | `CT_e` | `ct2` |
| 44 + `CT_e` | `CT_s` | `ct3enc` |
| … | 0 + 16 | `empty` |
| … | 16 | `m1` |
| … | 16 | `m2` |

**Size:** `92 + CT_e + CT_s` bytes of UDP payload.

### 2.3 CookieReply (type 0x03)

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | Header |
| 4 | 4 | `sidi` of the InitHello being answered |
| 8 | 24 | `nonce`: random |
| 32 | 16 + 16 | `cookie`, encrypted (§6.2) |

**Size:** 64 bytes, as in WireGuard.

### 2.4 Sizes for each candidate parameter set

The UDP payload sizes below come from the layouts above. The "+48" columns add IPv6 (40) and UDP (8) headers.

| `EPHEMERAL_KEM` | `STATIC_KEM` | InitHello payload | InitHello +48 | RespHello payload | RespHello +48 | Fits 1280 without fragmentation? |
| --- | --- | --- | --- | --- | --- | --- |
| ML-KEM-512 | mceliece348864 | 1044 | 1092 | 956 | 1004 | Yes |
| **ML-KEM-512** | **mceliece460896** *(analysed)* | **1104** | **1152** | **1016** | **1064** | **Yes** |
| ML-KEM-768 | mceliece348864 | 1428 | 1476 | 1276 | 1324 | No |
| ML-KEM-768 | mceliece460896 | 1488 | 1536 | 1336 | 1384 | No |
| ML-KEM-512 | ML-KEM-512 | 1716 | 1764 | 1628 | 1676 | No |
| ML-KEM-512 | ML-KEM-768 | 2036 | 2084 | 1948 | 1996 | No |
| ML-KEM-768 | ML-KEM-768 | 2420 | 2468 | 2268 | 2316 | No |

**Cross-check against the paper (B-00):**
- Our RespHello (1064) matches the paper's Table 8 exactly.
- Our InitHello is **4 bytes smaller** than the paper's 1156. The estimates in [candidate-analysis.md](candidate-analysis.md) §5 are 4 bytes larger for the same reason.
- The reviewer is asked to identify the missing 4 bytes, for example a field width we have read differently. Until then, the larger figure is used for MTU decisions.

## 3. Session identifiers

| Rule | Source / status |
| --- | --- |
| `sidi` and `sidr` are 32-bit values chosen at random by the side that will **receive** packets addressed to it | [WG] |
| A side never has two live keypair slots with the same identifier. On a collision it draws again | [WG]; **B-15** |
| Identifiers are not secrets and not authenticators. They only select state. A packet whose identifier matches no live slot is dropped with no reply | [WG] |

## 4. Transport (data) packet, type 0x04

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | Header |
| 4 | 4 | `receiver`: the session identifier the receiving side chose |
| 8 | 8 | `counter`: 64-bit packet number, little-endian |
| 16 | N + 16 | AEAD ciphertext of the padded inner IP packet |

**Encryption rules:**

| Item | Specification | Source / status |
| --- | --- | --- |
| Key | `tki` from device to server, `tkr` from server to device ([handshake.md](handshake.md) §6). One key per direction | [HWG] |
| Data cipher | Named parameter `DATA_AEAD`: **AES-256-GCM** (candidate D-010, preferred) or **ChaCha20-Poly1305**. One per protocol identifier | **B-17** |
| Nonce (96 bits) | 4 zero bytes ‖ `counter`, as WireGuard does for ChaCha20-Poly1305 (RFC 8439). For AES-256-GCM this layout is the deterministic construction of SP 800-38D §8.2.1: fixed field zero, 64-bit invocation field. It is safe because every key is used in one direction only and never again after its session | [WG] + [STD]; **B-17** |
| Counter | Starts at 0 for each new key; +1 for every packet sent; **never reused, never decremented**. Sending stops before the per-key limit ([key-lifecycle.md](key-lifecycle.md) §4) | [WG] |
| Associated data | The 16-byte header (`type`, `version`, `reserved`, `receiver`, `counter`) | **B-16.** WireGuard uses empty associated data and relies on the nonce and key selection. Authenticating the header as well is standard AEAD practice (as in QUIC) but differs from the analysed source |
| Padding | Plaintext padded with zeros to a multiple of 16 bytes, never beyond the tunnel MTU (§5) | [WG]; **B-24** |
| Keepalive | A transport packet with an empty plaintext | [WG] |

**Receiver order of checks:**
1. Length is at least 32 bytes, and header checks pass.
2. `receiver` selects a live slot; otherwise drop.
3. `counter` is not below the replay window and not already seen (§4.1). This is a **cheap pre-check, with no state change**.
4. Decrypt and verify the AEAD. On failure, drop and count it toward the integrity limit ([key-lifecycle.md](key-lifecycle.md) §4).
5. **Only now** mark `counter` as seen and advance the window. Only authenticated packets change replay state (RFC 6479 practice).
6. Inner-packet check: the decrypted packet's **source address must be one assigned to this device**; otherwise drop. This is WireGuard's "cryptokey routing" (**B-22**). Then apply the device's authorization (S-09, NP-G1).
7. Roaming update (§7).

### 4.1 Anti-replay window

| Item | Specification | Source / status |
| --- | --- | --- |
| Method | Sliding bitmap window as in RFC 6479 | [STD] |
| Window size | 8192 packet numbers. This is why the per-key limit is `2^64 − 2^13 − 1`, as in WireGuard | **B-18** |
| Older than the window | Drop | RFC 6479 |
| Duplicate inside the window | Drop | RFC 6479 |

## 5. MTU and fragmentation

### 5.1 Handshake messages

- For the analysed parameter set, every handshake message fits a 1280-byte IPv6 path (§2.4). **SVP never relies on IP fragmentation.** Handshake datagrams are sent with "don't fragment" where the platform allows it.
- If the chosen parameter sets do **not** fit, §5.3 applies. That is the only case where SVP fragments.

### 5.2 Tunnel (inner) MTU

**Overhead per packet:**
- outer IPv4 20 or IPv6 40;
- UDP 8;
- SVP header 16;
- AEAD tag 16.

That is 60 bytes over IPv4 and 80 bytes over IPv6.

| Outer path MTU | Inner MTU (IPv6 outer) | Note |
| --- | --- | --- |
| 1500 | 1420 | WireGuard's default |
| 1280 (IPv6 minimum) | 1200 | Too small to carry **IPv6 inside** the tunnel, which needs 1280 |

**Rules:**
- The default inner MTU is 1420, and it can be configured.
- If the outer path cannot carry `inner MTU + 80`, packets are dropped, never fragmented by SVP.
- Status must make such a path visible as "reachability failing", not "connected" (D-020).
- **Path MTU discovery on the outer path is OPEN** for M3 and later.

### 5.3 Handshake fragmentation (only if a non-fitting parameter set is chosen)

**Status: BLOCKED B-23. Not specified, by design**, until M2-Q3 is decided. If it is ever needed, these requirements apply:
- **cookie first:** no reassembly state is kept for any source that has not returned a valid `m2` cookie (§6);
- at most **one** reassembly buffer per validated source address, a fixed global number of buffers, a short timeout and a fixed fragment count;
- fragments are not authenticated until the whole message is checked, so the design must show that forged or dropped fragments can only cause a retry, not a state change;
- the reviewer must confirm that splitting does not change the analysed message contents.

## 6. Pre-authentication protection and anti-amplification (Septuvo's own design, BLOCKED B-06, B-19, B-20)

### 6.1 The cost problem

- To process an InitHello, the server must perform an X25519 operation, a **static-KEM decapsulation** and an AEAD decryption **before** it knows which device is calling ([handshake.md](handshake.md) §9.2).
- Anyone who knows the server's public keys can make it do this. The keys are given to every enrolled device, so they cannot be treated as secret.
- The aim is not to make that work free. The aim is to make sure that:
  - spoofed source addresses cost the server almost nothing;
  - real addresses are rate-limited;
  - existing sessions keep working under any handshake flood.

### 6.2 Cookie (`m2`) mechanism [WG, adapted]

| Item | Specification | Source / status |
| --- | --- | --- |
| Cookie secret `Rm` | 32 random bytes, replaced every 120 s; never stored on disk | [WG] |
| Cookie value | `cookie = MAC(Rm, source IP address ‖ source UDP port)`, 16 bytes. It proves the sender receives packets at that address | [WG] |
| CookieReply encryption | XChaCha20-Poly1305 with key `HASH(lbl4 ‖ HASH(Scr ‖ Spqr))`, a random 24-byte `nonce`, and associated data = the `m1` of the InitHello being answered. The device accepts a cookie only for an InitHello it actually sent, and stores it for 120 s | [WG] form; key input and `lbl4` **BLOCKED B-06**. XChaCha20-Poly1305 extends the [handshake.md](handshake.md) §4 primitive list (**B-05**) |
| Device behaviour | After a CookieReply, the device sends its **next** InitHello with `m2 = MAC(cookie, inner ‖ m1)` (handshake.md §5.1). It does not retransmit early because of the cookie | [WG] |

### 6.3 Server rules

| # | Rule | Status |
| --- | --- | --- |
| P1 | **Not under load:** process InitHellos normally (§9.2 of handshake.md), subject to P3 and P4 | B-19 |
| P2 | **Under load:** for an InitHello whose `m2` is missing or wrong, reply with **one** CookieReply and do **no other work**: no X25519, no decapsulation, no state. If `m2` is valid, continue to P3. Checking `m2` costs one MAC | [WG] + B-19 |
| P3 | **Per-source budget:** each source address (IPv4 /32; IPv6 /64, since one host may control a whole /64) has a token bucket for InitHellos that reach the expensive steps. When it is empty, drop and record `rate_limited` | B-19 |
| P4 | **Global budget:** a fixed number of decapsulation workers and a fixed-length queue. A full queue means "under load" (P2), and the oldest unvalidated work is dropped first | B-19 |
| P5 | **Existing sessions first:** transport packets for live sessions are processed on a path that handshake work cannot starve. Under load, new handshakes are refused; existing sessions are not (S-07) | B-19 |
| P6 | **No state for strangers:** no per-source state is kept before P2/P3 except the token bucket, and the bucket table has a fixed size with eviction | B-19 |
| P7 | **Logging under load:** reason codes are counted, not logged per packet ([state-machine.md](state-machine.md) §6) | B-19 |

Budget values (tokens per second, bucket size, number of workers, queue length, the "under load" threshold) are **not set here**. They are measured in the G2 lab on the reference hardware (Q11, D-018), and only then fixed.

### 6.4 Anti-amplification (B-20)

| # | Rule |
| --- | --- |
| A1 | The server **never sends more bytes to an address than it received from that address** in the message being answered. One InitHello produces at most one RespHello **or** one CookieReply, never both |
| A2 | Every candidate parameter set keeps RespHello ≤ InitHello and CookieReply (64 bytes) ≤ InitHello (§2.4). **A parameter set that broke this would require the device to pad InitHello**, as QUIC pads its first packet, and is otherwise not allowed |
| A3 | The server never initiates traffic to an address it has not received authenticated traffic from (B-01, §7) |
| A4 | Malformed, unknown-device and failed messages get **no reply at all** ([handshake.md](handshake.md) §9.4) |

### 6.5 Residual risk (stated, not hidden)

- An attacker with **many real addresses** (a botnet) can keep the server's decapsulation workers busy up to the global budget. This **delays or refuses new handshakes**, including for legitimate devices, while existing sessions continue (P5).
- This cannot be removed without device-specific secrets checked before decapsulation, which the analysed design does not provide (handshake.md §9.2).
- Accepted as a version 1 residual risk for the G1 reviewer to assess (threats.md R-01, ADV-03, Q12).

## 7. Roaming (endpoint changes)

| Item | Specification | Source / status |
| --- | --- | --- |
| Device moves | The device simply keeps sending from its new address; nothing special is needed | [WG] |
| Server follows | The server updates the device's endpoint to the source address of the **most recent transport packet that authenticated and advanced the replay window** (§4, step 5) | [WG] |
| **Known weakness** | An on-path attacker can copy a fresh, valid packet and resend it from another address first. The server would then send that device's downstream traffic to the attacker's chosen address: a redirection and a reflection vector (not a decryption) | WireGuard accepts this |
| Proposed mitigation (**BLOCKED B-21**) | Path validation before full use of a new address, in the style of QUIC (RFC 9000 §8.2). Until the device answers a challenge from the new address, the server sends no more bytes there than it has received from there (A1), and keeps the old address for everything else. A challenge needs a new message type, so the reviewer is asked whether the protection is worth the added complexity | **Reviewer question** |

## 8. Errors on the wire

- SVP sends **no error messages**. Every rejected datagram is dropped silently ([handshake.md](handshake.md) §9.4). The only responses to an unauthenticated datagram are a RespHello (to a valid InitHello) or a CookieReply (under load, P2).
- Local reason codes are in [state-machine.md](state-machine.md) §6.
- A **version mismatch** is not reported on the wire. The device shows "server not answering" after `REKEY_ATTEMPT_TIME`, and diagnostics suggest checking versions.

## 9. Coverage of the review checklist

| Item | Where | Status |
| --- | --- | --- |
| P-02 Protocol version and wire format | §1–§4 | Draft; **B-15, B-16** |
| P-07 Packet numbering and nonces | §4 | Draft; **B-17** |
| P-08 Replay and state | §3, §4.1, §7 | Draft; **B-18, B-21** |
| P-10 MTU and fragmentation | §5 | Draft; fragmentation **B-23** only if needed |
| P-11 Resource protection | §6 | Draft; **B-06, B-19, B-20**; residual risk §6.5 |
| P-12 Error semantics | §8 | Draft |
| P-09 Rotation and lifetime | [key-lifecycle.md](key-lifecycle.md) | Draft |

## 10. BLOCKED register, continued (handshake.md §12 has B-00 to B-14)

| ID | Item | Why it is not covered by an existing analysis |
| --- | --- | --- |
| B-06 (detail) | Cookie key input, `lbl4`, and the use of XChaCha20-Poly1305 for CookieReply (§6.2) | WireGuard's form, re-keyed to Septuvo's two-part server identity; not analysed |
| B-15 | Header layout, `type`/`version` values, little-endian integers, exact-size rule, identifier collisions | Septuvo layout |
| B-16 | Transport header used as AEAD associated data | Differs from WireGuard |
| B-17 | `DATA_AEAD` choice and the nonce layout for AES-256-GCM | The data phase is outside the paper's analysis; GCM usage limits differ from ChaCha20-Poly1305 ([key-lifecycle.md](key-lifecycle.md) §4) |
| B-18 | Replay window size and method | Values from WireGuard and RFC 6479; not analysed for SVP |
| B-19 | Under-load rule, budgets, scheduling (P1–P7) | Septuvo's own design |
| B-20 | Anti-amplification rules (A1–A4) | Septuvo's own rules |
| B-21 | Roaming and path validation | Known weakness in the source design; mitigation is new |
| B-22 | Inner source-address check | WireGuard behaviour; not part of the analysed handshake |
| B-23 | Handshake fragmentation (only if needed) | New, and unspecified on purpose |
| B-24 | Padding policy | WireGuard behaviour; traffic-analysis value not analysed |

## Sources

- WireGuard whitepaper §5.4 (message formats), §5.3 (cookies), §6 (timers): <https://www.wireguard.com/papers/wireguard.pdf>
- Lafourcade et al., USENIX Security 2025 (Table 8 sizes): <https://www.usenix.org/system/files/usenixsecurity25-lafourcade.pdf>
- RFC 6479, IPsec anti-replay algorithm without bit shifting: <https://www.rfc-editor.org/rfc/rfc6479>
- RFC 8439, ChaCha20-Poly1305: <https://www.rfc-editor.org/rfc/rfc8439>
- NIST SP 800-38D, GCM (§8.2.1 deterministic nonces): <https://csrc.nist.gov/pubs/sp/800/38/d/final>
- RFC 9000, QUIC (§8.2 path validation, §17 header bit): <https://www.rfc-editor.org/rfc/rfc9000>
- RFC 9443, multiplexing on one port: <https://www.rfc-editor.org/rfc/rfc9443>
