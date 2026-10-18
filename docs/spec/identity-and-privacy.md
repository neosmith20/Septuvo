# Septuvo — identity, enrollment, revocation and privacy requirements (M1-03)

**Status:** requirements draft for owner review. It states **what** identities, invitations, revocation and logging must guarantee. It does not choose algorithms or message formats; those belong to M2 and the G1 review (D-012 and D-024 stay candidates, D-013 stays blocked). Owner questions are marked **Q*n*** (section 9).
**Builds on:** [threats.md](threats.md) (ADV-05, ADV-06, ADV-07, ADV-10, ASM-04, ASM-05), S-03, S-10, S-11 and S-13 in [SECURITY_MODEL.md](../SECURITY_MODEL.md), and D-007 (standalone) and D-017 (no mandatory telemetry).

## 1. Identities

| ID | Requirement |
| --- | --- |
| ID-1 | Every device and every Septuvo Server has its own long-term identity key pair, generated **on that machine** from the operating system's random number generator (ASM-02). |
| ID-2 | A device's private identity key never leaves the device: not in invitations, QR codes, backups, logs or diagnostics. The only possible exception is an explicit, encrypted server identity backup (Q8). |
| ID-3 | An identity is its public key, shown to people as a short fingerprint. Algorithms are chosen in M2 (D-012 candidate). |
| ID-4 | No two devices share a key; reinstalling or replacing a device creates a new identity. |
| ID-5 | The specification (M2-02) must allow an identity key to be replaced and the old one retired without weakening authentication. |

## 2. Trust model (personal profile, version 1)

- **The user's Septuvo Server is the trust anchor for its devices.** It keeps the list of enrolled device identities and what each may reach.
- **Each device pins its server's identity** at enrollment (ASM-04). A later change of server identity is refused with a clear warning and is never accepted automatically (RV-5).
- **No certificate authority, account or online directory** in version 1 (D-007). Organization PKI and single sign-on are M9.

## 3. Enrollment

### 3.1 The invitation

An invitation is created by the administrator on the server (section 6) and handed to the new device as a QR code or a text link. It contains **only**:

| Field | Purpose |
| --- | --- |
| Server endpoint(s): address and UDP port | Where to connect (no underlay DNS needed; network-policies.md NP-U3) |
| Server identity fingerprint (public) | Lets the device authenticate the server during enrollment |
| One-time enrollment secret | Proves the device holds a valid invitation |
| Expiry time (server clock) | Limits how long a leaked invitation is useful |
| Scope | What the new device will be allowed to reach (home network, internet exit or both) |

**An invitation never contains a private key** (S-10). The exact encoding and the size of the secret are set in M2 and reviewed at G1.

### 3.2 Invitation rules

| ID | Requirement |
| --- | --- |
| EN-1 | **Single use.** The secret is consumed atomically by the first successful enrollment; parallel attempts cannot both succeed. |
| EN-2 | **Expiry** is checked against the server's clock only (ASM-05). Default lifetime and maximum: Q7. |
| EN-3 | The administrator can list and cancel unused invitations. |
| EN-4 | The server stores only a verifier from which the secret cannot be recovered (construction in M2), never the secret itself. |
| EN-5 | Failed attempts are limited per invitation and per source; after the limit the invitation is cancelled. |
| EN-6 | The device enrolls only over a connection on which the server proved the identity in the invitation; on mismatch it stops and reports a possible interception. |
| EN-7 | **Administrator confirmation** (Q7): the server shows the pending device with a short code that the device also displays; the administrator approves. This defeats a stolen invitation (ADV-06). |
| EN-8 | The device creates its identity key (ID-1) before contacting the server and sends only its public key, its proof of the invitation and a device name. |
| EN-9 | Apps treat an invitation as a secret: shown only on request, never logged, hidden once used or expired. |

### 3.3 Enrollment flow (requirement level)

1. The administrator runs an invite command on the server, choosing the scope and optionally a device name.
2. The new device scans the QR code or opens the link, and generates its identity key.
3. The device connects to the endpoint and authenticates the server against the fingerprint (EN-6).
4. The device proves the invitation and sends its public key (EN-8).
5. If confirmation is on (EN-7), both sides show the same short code and the administrator approves.
6. The server records the device identity and scope and consumes the invitation (EN-1). The device stores the pinned server identity.
7. The device can now connect. Status shows "enrolled", not "connected", until the four stages pass (D-020).

## 4. Key storage

| ID | Requirement |
| --- | --- |
| KS-1 | Private keys are stored in the strongest store the platform offers for that algorithm. On Linux (version 1) this is a file readable only by the Septuvo service account (mode `0600`, local disk). TPM sealing on Linux is **OPEN** ([platform-feasibility.md](platform-feasibility.md)). |
| KS-2 | Where hardware key storage cannot hold the chosen algorithm, the private key is encrypted with a key that the hardware **does** hold, where one exists. The protection actually achieved is stated per platform. Nothing is described as hardware-backed unless it is. |
| KS-3 | Private keys are excluded from operating-system and cloud backups where the platform allows it. |
| KS-4 | The unprivileged app (TB-1) never receives private keys or session secrets. |
| KS-5 | Secret erasure from memory is best effort and its limits are documented (S-03). Crash dumps are local only and must not be uploaded automatically (PR-6). |

**Missing platform capabilities (called out, to be verified in [platform-feasibility.md](platform-feasibility.md) and M7):**

- **Post-quantum keys in hardware stores.** Common hardware key stores (TPM 2.0, Apple Secure Enclave, Android Keystore and StrongBox) are not known to support ML-KEM or ML-DSA keys today. Apple's Secure Enclave is limited to P-256 keys.
- **Classical keys.** Whether X25519 or Ed25519 keys can live in each store also varies and is unverified.
- **Consequence:** expect KS-2 wrapping or software-only storage for the identity keys, disclosed per platform.

## 5. Revocation, recovery and policy lifetime

| ID | Requirement |
| --- | --- |
| RV-1 | The administrator can revoke a device on the server. New handshakes are refused at once, and the device's existing sessions are closed and its traffic dropped at once on that server (network-policies.md NP-G5). "At once" is measured in the G2 lab. |
| RV-2 | The revocation list survives restarts. If the server cannot read its authorization data it refuses all devices rather than accepting a revoked one. |
| RV-3 | **Lost or stolen device** (ADV-05): revoke it, then enroll the replacement with a new invitation. Device keys are never recovered. |
| RV-4 | A device suspected of compromise (ADV-07) is handled like a lost one. |
| RV-5 | **Lost or compromised server key:** every device must pin the new identity, normally by re-enrolling (or from a backup, Q8). Devices show "server identity changed — not connecting" and never accept the change silently. |
| RV-6 | **Policy lifetime.** In version 1 the server decides authorization for every packet it forwards (NP-G1), so a device cannot keep access after revocation by caching anything. Sessions re-authenticate within a bounded lifetime set in M2 (S-03). Multi-gateway propagation bounds are M9 (**OPEN**). |

## 6. Administration and the local control boundary

| ID | Requirement |
| --- | --- |
| AD-1 | Version 1 has **no network-exposed administration API**. The server is administered with a local command-line tool on the server host that needs administrator rights. |
| AD-2 | Administrative actions are logged locally (who, when, what) without secrets (PR-2). |
| AD-3 | Devices cannot change their own scope or the server's authorization data. |
| AD-4 | On a device, the privileged service (TB-1) accepts changes to policy, LAN exceptions, connection and disconnection only from users the OS identifies as authorized, or after interactive confirmation. Any local user may read status. Inputs are validated as hostile (ADV-10). |

## 7. Privacy, logging and telemetry

| ID | Requirement |
| --- | --- |
| PR-1 | **No telemetry** (D-017). Any future metrics are opt-in, disclosed and off by default. |
| PR-2 | Logs never contain private keys, session secrets, invitation secrets, packet contents or DNS query names, and do not record which sites were visited. |
| PR-3 | Device logs may contain status-stage changes, errors, the server endpoint, timestamps and device names. Retention defaults: Q10. The user can set retention to zero. |
| PR-4 | Server logs may contain per-device connect and disconnect times and the device's public source address. Destinations of exit traffic are not logged. Retention defaults: Q10. |
| PR-5 | Diagnostics export is started only by the user, is shown before it is saved, removes secrets, and is sent by the user personally. There is no upload service. |
| PR-6 | Crash reports stay on the machine; nothing is uploaded automatically. |

## 8. Local-only operation (D-007)

| ID | Requirement |
| --- | --- |
| LO-1 | A device and its server work with no account, no Septuvo-run service and no internet connection to any third party. Enrollment works on a local network alone. |
| LO-2 | Septuvo makes no network connections except to the user's own server(s), unless the user turns on or starts something that needs one: update checks (Q9 as amended by D-042: asked once at first-run setup, On preselected, nothing contacted before the answer), or a diagnostic ping of a public host that the user starts. The public IP address shown in Advanced mode comes only from the user's own server, never from a third-party lookup. |
| LO-3 | Security checks do not depend on accurate client clocks (ASM-05). |

## 9. Owner questions (this document)

Full explanations, options and trade-offs: [M1-owner-questions.md](../reviews/M1-owner-questions.md). **Decided 2026-10-15: every recommendation below was adopted (D-032).**

| Q | Question | Decision (as recommended) |
| --- | --- | --- |
| Q7 | Invitation lifetime, and administrator confirmation (EN-7) | Default lifetime 24 hours, maximum 7 days, at most 5 failed attempts. Confirmation **on** by default. |
| Q8 | Server identity backup | Offer an encrypted, offline export that the administrator starts manually; off unless used. Without it, a lost server means re-enrolling devices. |
| Q9 | Update checks | Off by default; ask once during setup. No other network calls. **Amended 2026-10-18 (D-042):** asked once at first-run setup with On preselected; owner to confirm (desktop-ui-reconciliation.md, open decision 2). |
| Q10 | Log retention | 7 days on devices and server, metadata only (PR-3, PR-4), configurable down to zero. |
