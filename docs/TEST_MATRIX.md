# Septuvo — test and release qualification matrix (planned)

These tests are **requirements**, not performed results. No test has passed merely because this file exists.

## T0 — docs/spec correctness (M0–M2)

- Terminology and four official product names consistent in README, docs and UX copy.
- All cryptographic choices in candidate/open state until reviewer and owner approve.
- Work order includes changed-file scope, reproducible acceptance criteria and rollback.
- Protocol fixtures, state machine and packet grammar unambiguous before G1.

## T1 — protocol parser and handshake (after G1)

- Known-answer and interoperability vectors, real entropy in production path.
- Malformed/short/truncated/oversized/fuzzed packets never panic, escape bounds or allocate without limit.
- Corrupt tag/signature/ML-KEM ciphertext, wrong identity, expired enrollment, version mismatch, replay and downgrade attempts fail closed.
- Loss/reordering/duplicates during handshake, key rotation, session migration and restart do not reuse AEAD nonce or bypass identity.
- Concurrent packet workers cannot race counters/epochs; crash recovery cannot replay stale key state.
- Capture and inspect packets to verify no plaintext data goes on untrusted interface.

## T2 — routing, DNS and kill switch (after lab tunnel)

- IPv4 + IPv6 protected full tunnel, local approval off/on, custom split prefixes and overlapping CIDRs.
- Distinguish /0 from paired /1 IPv4 routes; test LAN exceptions independent of VPN underlay exceptions.
- Run DHCP, DNS, DoH/DoT where relevant, IPv6 RA, interface roam, sleep/wake, reconnect and service crash with leak detection.
- Disconnected tunnel blocks protected traffic, without disabling authorized local exceptions.
- Remote home-network and local direct NAS/Jellyfin access: ensure intended paths and no accidental route leakage.
- Gateway has destination authorization separate from successful authentication.

## T3 — cross-platform UX and packaging

- Fresh signed install, enroll, reject, revoke, disconnect, update and uninstall with route/firewall cleanup.
- GUI on 1920x1080 and smaller resizable windows: layout scroll, no inaccessible controls.
- Close/minimize to tray preserves intended tunnel state; explicit disconnect works and is not silently triggered by closing UI.
- Test statuses separately: interface ready, peer authenticated, data reachable, DNS resolved.
- Informative failures for wrong endpoint, blocked UDP, invalid identity, policy reject, DNS failure, network conflict.
- Windows, Linux, macOS, Android, iOS only marked supported when actual OS-integrated tests pass; document platform exceptions.

## T4 — performance and stability

- Publish hardware/OS versions, topology, MTU, link baseline, latency distribution, packet sizes, CPU architecture and acceleration settings.
- Throughput in both directions (Gbps and packets/s), P50/P95/P99 latency, CPU/memory under load, tail drop/loss/reordering.
- Compare the same host/kernel/network against alternatives using equivalent security/configuration and repeatable method; no exaggerated speed claims.
- Long-running soak, process restart, Wi-Fi/cellular roam, battery draw, hundreds/thousands of connected *and* active sessions measured separately.

## T5 — enterprise and multi-tenant

- Isolation across tenants even with overlapping address space, revoked devices, invalid routing claims and rogue admin sessions.
- Bounded pre-auth CPU/memory and handshake amplification, fairness under abusive tenant traffic.
- Control-plane loss/recovery, gateway draining/failover, stale policy time limits, guaranteed revocation bound.
- Signed configuration propagation, audit attribution, HA failover, backup/restore, disaster recovery.
- Validate only measured capacities and explicitly list hard limits.

## Evidence requirement

Every test record must include test ID, commit SHA, command/test harness, environment, output/artifacts, expected vs actual outcome, and PASS/FAIL/NOT RUN. A static analysis pass never counts as a live network test.

**Release stop:** any key/nonce reuse, unauthenticated connection acceptance, policy bypass, protected-route/DNS leak, cross-tenant break or unreviewed handshake change blocks release.
