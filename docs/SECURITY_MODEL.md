# Septuvo — threat model and non-negotiable security requirements (draft)

**Draft pending external specialist review.** It is NOT an independent audit or a complete cryptographic security proof.

## Assets

- Device long-term identity private keys, ephemeral session secrets, symmetric traffic keys, enrollment invitations.
- User packet plaintext, DNS requests, destination metadata and traffic routing policies.
- Control-plane authorization policy, tenant membership, device revocation state.
- Privileged network/firewall/DNS configuration and the integrity of signed updates.

## Assumed attackers

- Active on-path attacker on any attached network: intercepts, modifies, delays, drops, replays or forges traffic; controls rogue Wi-Fi, DNS or a local gateway.
- Internet attacker probing an exposed UDP port and attempting amplification, memory/CPU exhaustion or malformed-packet exploitation.
- Stolen/lost client device, compromised enrollment token, rogue or malicious peer.
- Compromised management credential/control-plane node; malicious tenant in shared provider infrastructure.
- Attacker storing encrypted traffic for later quantum cryptanalysis.
- Untrusted dependency/update artifacts and privileged local application abuse.

## Out of scope as blanket guarantees

- Endpoint device compromised by malware/root/admin: a VPN cannot protect plaintext at that endpoint.
- Services/applications with insecure authentication; local-LAN exceptions do not magically provide end-to-end confidentiality.
- Total secrecy of IP addresses, traffic timing or packet sizes, and anonymity from an egress operator.
- Protection against every future quantum or implementation attack.
- Availability against an unlimited-volume upstream network flood.

## Required security properties

**S-01 Authenticated key agreement:** peers authenticate identities, validate key material and bind roles, algorithms, protocol version and all relevant negotiation into a reviewed transcript. Candidate hybrid construction must account for compromise of either component, cross-protocol use and peer impersonation.

**S-02 Quantum resistance goal:** protect recorded session traffic against plausible future quantum attacks via independently reviewed hybrid construction and authenticated post-quantum-capable identity. Avoid relying only on classical signatures for long-lived security without an explicit justified security policy.

**S-03 Key lifecycle:** hardware-backed storage where available, credible entropy, unique per-direction keys, forward secrecy, secure rekey/rotation/revocation, lifecycle across process crashes and restarts. Define exact secret-erasure limitations.

**S-04 AEAD safety:** formally specified per-key nonce uniqueness, packet-number and epoch handling, verified associated data, ciphertext length limits and replay windows. A retransmission or process restart cannot reuse an AES-GCM key/nonce combination.

**S-05 Downgrade safety:** no unauthenticated algorithm/version fallback. Explicit policy for unsupported peers and cryptographic agility.

**S-06 Replay and endpoint changes:** authenticated anti-replay windows, safe rebinding, stable connection ownership and protection against session hijack.

**S-07 DoS:** size limits, bounded memory per unauthenticated peer, rate limits, address validation / anti-amplification, packet/handshake loss behavior and fair resource limits for multiple tenants.

**S-08 Hostile networks:** full-tunnel default protected IPv4/IPv6; strict fail-closed firewall semantics; prevent routing and DNS leaks after reconnect, roaming, crashes, sleep and updates. Underlay exceptions must be minimal and auditable.

**S-09 Authorization:** “authenticated peer” is not “permitted destination.” Least-privilege policy independently gates access to LAN peers/subnets; isolation enforced at the gateway, not UI alone.

**S-10 Enrollment:** verify server/gateway identity during enrollment; expiring scoped invitations, secure device key creation, revocation and key replacement. No exported private keys in QR-code payloads.

**S-11 Control plane:** separation from data plane, authenticated scoped operations, explicit admin privileges, bounded policy cache lifetime, tenant partitioning and audit logging without sensitive payloads.

**S-12 Updates and dependencies:** signed/verifiable builds, dependency review, SBOM, no malicious silent remote configuration changes, secure install/uninstall and recovery.

**S-13 Diagnostics/privacy:** distinguish transport up, authenticated session, reachability and DNS; sensitive traffic/credentials never logged. Opt-in metrics only and disclosed retention.

## Security gates / evidence

Before real SVP crypto code (**G1**): a full wire specification, exact KEM/signature choices, formal identity model, KDF schedule, transcript, AEAD nonce/epoch arithmetic, replay model, fragmentation/amplification controls, error handling and threat analysis must be reviewed by a qualified external protocol/security expert and approved by the owner.

Before external pilot (**G2**): lab tunnel interoperability and negative tests (malformed packets, corrupted tags, forged peers, stale replay, downgrade attempts, loss, rekey, migration), crash and leak tests. These are lab results, **not** a security audit.

Before production/public claims (**G3**): independent audit of protocol and implementation, documented findings and remediation, cross-platform OS policy validation, signed update/release procedures, vulnerability reporting and incident plan.

Before large-provider claims (**G4**): actual load and isolation tests, control-plane failover, revocation latency, rate-limiting under attack and proved tenant separation.

**Stop condition:** if a requirement cannot be met on a proposed platform, do not silently weaken it. Record limitations and block that platform/release until an owner-approved supported solution exists.
