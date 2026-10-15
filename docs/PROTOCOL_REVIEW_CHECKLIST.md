# SVP — protocol design / external review checklist

**No executable real-handshake implementation before G1.** The name “SVP/1” does not imply the format or version is frozen. This checklist is a request for a real specification, **not permission for an agent to fill gaps**.

## Specification artifacts required (each must link to a reviewed document)

| ID | Deliverable | What reviewers must see |
| --- | --- | --- |
| P-01 | Threat and trust model | Assets, active/on-path/compromised peer, identity trust and quantum threat |
| P-02 | Protocol version & wire format | Exact byte layouts, sizes, grammar, authenticated fields, reserved values, forward compatibility |
| P-03 | Authenticated identity/enrollment | Long-term vs ephemeral keys, signature semantics, authority, verification, revocation and recovery |
| P-04 | Hybrid handshake | Ephemeral classical and post-quantum KEM roles (candidate D-011: X25519 + ML-KEM-768); exact public-key/ciphertext validation; combiner construction and proofs; key confirmation |
| P-05 | Transcript and KDF | KDF choice (candidate D-024: HKDF), exact input encodings, domain separation, transcript hash, extract/expand steps, salt/labels, key separation |
| P-06 | Cipher suites | Exact AEAD choices, key lengths, authenticated negotiation, policies for failure/downgrade |
| P-07 | Packet numbering | Exact nonce derivation, uniqueness under concurrency, retransmission, crash and failover |
| P-08 | Replay & state | Session IDs, direction/epoch, bounded replay windows, resumption/migration, duplicate handling |
| P-09 | Rotation & lifetime | Traffic key limits, expiration, rekey sequence, overlapping epochs, lost peers, zeroization |
| P-10 | MTU & fragmentation | UDP payload limits, handshake fragmentation/reassembly, malicious memory caps and retransmits |
| P-11 | Resource protection | Anti-amplification, address validation, rate limits, parsing budgets, unauthenticated computation |
| P-12 | Error semantics | Fail-closed state transitions, timeouts, disconnect, reconnect, version mismatch |
| P-13 | Wire fixtures | Before G1: documentation-only test formats, negative cases and a conformance plan; per owner decision D-021 (option C), computed vectors are deferred until after G1. **No Septuvo executable crypto/protocol code or generator before G1.** Qualified reviewer must accept pre-G1 evidence as sufficient or block G1. After G1: implement and verify canonical computed test vectors, invalid cases and deterministic test fixtures isolated from production RNG |
| P-14 | Protocol analysis | Review record, formal/model-checking artifacts if feasible, known assumptions and residual risks |

## Acceptance / prohibition

- Review the full construction; a citation to an RFC about TLS alone is not a security review of this independent protocol.
- Reject ambiguous field lengths, unsigned negotiation, circular trust, unauthenticated peer metadata, non-unique AEAD nonces, or unlimited pre-auth allocation.
- Reject undocumented opt-outs of post-quantum or authentication requirements and unauthenticated 0-RTT user data.
- Publish a decision record for each contentious area, including alternatives rejected and rationale.
- Have the qualified reviewer sign off on the precise revision/commit and on whether prototype implementation can proceed; owner approves G1 separately.
- If the protocol changes materially after review, **reopen G1** for that part of the specification.

Status: **G1 has NOT passed, and no item is reviewed or approved.** Draft answers to P-01 to P-13 are mapped in [conformance-plan.md](protocol/conformance-plan.md) §1; P-14 is an OPEN finding. Computed vectors are NOT RUN (D-021).
