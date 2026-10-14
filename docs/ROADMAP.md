# Septuvo — owner-controlled engineering roadmap

**Status: initial blueprint. Not an authorization to code or a schedule.** Follow [AGENTS.md](../AGENTS.md), [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md), [TASK_CATALOG.md](TASK_CATALOG.md), [TRACEABILITY.md](TRACEABILITY.md) and the [work-order template](WORK_ORDER_TEMPLATE.md). This file defines stages; the task catalog defines **exact per-task inputs, outputs and verification**. If any documents conflict, stop and record the ambiguity—do not choose independently. Milestones advance **in the order below**, with exactly one narrowly scoped task per agent checkpoint. Owner must explicitly approve every new gate and work order. An agent must never continue to the next milestone because time remains.

## Gate summary

| Gate | Meaning | Authoritative state |
| --- | --- | --- |
| G0 | Owner approves scope, decisions, non-goals and this roadmap | [STATUS.md](STATUS.md) |
| G1 | Outside specialist reviews exact SVP design; owner approves prototype implementation | [STATUS.md](STATUS.md) |
| G2 | Lab interop, negative cases, no leaks and controlled faults; **not external pilot permission** | [STATUS.md](STATUS.md) |
| G3 | Independent security audit, remediation, signed release builds, platform validation | [STATUS.md](STATUS.md) |
| G4 | Provider isolation, availability, measured capacity and operational controls | [STATUS.md](STATUS.md) |

Only owner-edited, dated ledger entries can pass a gate. All gates are initially PENDING.

## M0 — repository baseline and owner sign-off (FIRST)

**Goal:** a coherent blueprint before any cryptographic/network implementation.

- **M0-01:** Audit README, license, AGENTS.md, SECURITY.md, roadmap, task catalog, STATUS ledger, agent operations handbook, work-order template, architecture, security model (threat model), decision register, traceability, test matrix and protocol checklist for contradictions and claims. Documentation only. If defects exist, owner authorizes conditional **M0-01.1** corrections and accepts them before M0-02.
- **M0-02:** Resolve each open scope/implementation-language/platform question with the owner; record approvals/rejected alternatives in DECISIONS.md. The four names in D-001 are already owner-confirmed.
- **M0-03:** Agree on a staged review plan, external protocol-review expertise and practical test hardware. Define version-control/PR practices and review owners.

**Acceptance:** docs link correctly; every security requirement S-01…S-13 and every product requirement in TRACEABILITY.md has a validation location; no candidate crypto listed as an approved construction; brand/domain clearance marked pending; written G0 owner approval recorded. **STOP until G0.**

## M1 — threat model and requirements

- **M1-01:** Enumerate trust boundaries, hostile network scenarios, endpoint compromise assumptions, administrator compromise, quantum recording and required security guarantees.
- **M1-02:** Specify personal/home network journeys and zero-trust routing, LAN exceptions, IPv4/IPv6, conflicting routes, kill switch, DNS and acceptable failure modes.
- **M1-03:** Define privacy handling, secret storage, revocation, accountless enrollment and optional control-plane trust models.
- **M1-04:** Build a requirements-to-test traceability matrix and identify OS-specific unsupported cases.

**Acceptance:** S-01..S-13 mapped to protocols, components and T-tests; unresolved decisions identified; owner approves requirements. **No packet protocol code.**

## M2 — formal SVP specification (G1)

- **M2-01:** Compare existing reviewed constructions, Rust crypto library maturity, protocols/standards and licensing. Propose suites with alternatives and explicit dependency risks.
- **M2-02:** Draft exact full handshake and identity protocol: transcript, mutual authentication, trust roots, KEM validation/combiner, KDF schedule (HKDF candidate, D-024), key confirmation, errors and downgrade behavior.
- **M2-03:** Draft packet grammar, AEAD nonce/replay semantics, key epochs, rotation limits, retransmission, migration, reassembly, MTU and anti-amplification controls.
- **M2-04:** Draft **documentation-only** test-vector *formats*, state diagrams and conformance cases. No computed vectors, generator/property-test code, crypto/protocol code or live networking before G1. Per D-021 (option C), computed vectors are deferred until after G1; the external G1 reviewer must assess whether the documentation-only plan is sufficient.
- **M2-05:** Obtain qualified external cryptographic/protocol review, log findings, remediate and obtain owner G1 sign-off on exact specification revision.

**Acceptance:** every P-01..P-14 item answered, traceable, independently reviewed and owner-approved. Reopen G1 on material changes. **STOP until G1.**

## M3 — isolated Rust foundation (lab-only)

- **M3-01:** Create Rust workspace, formatting/lint/test/dependency/secret-scan CI, MSRV policy, license/SBOM checks and reviewable crate boundaries. No networking until specifically scoped.
- **M3-02:** Implement reviewed protocol parsing and serialization with bounded allocation, invalid-input tests, fixtures and fuzz targets.
- **M3-03:** Integrate approved cryptographic dependency behind narrow Rust APIs with reviewed test vectors, secure randomness and key-handling tests.
- **M3-04:** Implement reviewed handshake/session states and anti-replay with loss, concurrency, downgrade, interruption and key-epoch tests.

**Acceptance:** deterministic local interoperability tests against reviewed vectors; malformed traffic never bypasses checks; all tests recorded. **Not production-ready.**

## M4 — Linux-to-Linux SVP tunnel prototype

- **M4-01:** Implement userspace TUN/UDP packet path with privilege boundaries; only on isolated Linux test VMs/namespaces.
- **M4-02:** Carry IPv4 then IPv6 over mutually authenticated sessions using reviewed cipher/nonce handling; isolate test traffic.
- **M4-03:** Verify key rollover, packet reordering/loss, safe restart, MTU behavior, peer identity failures and no plaintext leakage.
- **M4-04:** Capture repeatable performance baseline and compare against direct UDP/IP path, with no promotional claims.

**Acceptance:** two lab nodes route test packets, authenticated status shown correctly, negative cases pass, host reverts cleanly after crash/uninstall. No internet-facing deployment. **STOP at controlled checkpoint.**

## M5 — policies, reliability and G2 lab qualification

- **M5-01:** Implement OS policy abstraction, explicit route decisions and LAN exception validation without trusting attached network.
- **M5-02:** Implement IPv4/IPv6 fail-closed kill switch, DNS policy and rollback/repair after service crash/restart.
- **M5-03:** Validate home local-NAS and remote home gateway flows and alternate internet exit without subnet/DNS leaks.
- **M5-04:** Run packet fault injection, fuzzing, long soak, roaming (where available), privilege, replay, fragmentation and high-loss tests.
- **M5-05:** Produce G2 evidence packet (T1/T2 tests, known limits and owner decision).

**Acceptance:** hostile-network, leak and negative test criteria pass on lab platforms; no claim of independent audit. **STOP until G2.**

## M6 — standalone Septuvo Server + guided onboarding

- **M6-01:** Implement local-only enrollment and device identity management, recovery/revocation, protected local state and self-hosting with zero vendor account requirement.
- **M6-02:** Implement lab-installable **unsigned pre-release** Linux server packaging with safe setup, service persistence, rollback and uninstall; document signing plan. **M8-03 owns signed production release artifacts**.
- **M6-03:** Provide CLI for admins plus simple role-based wizard; real connection diagnostics separate interface / authentication / data reachability / DNS.
- **M6-04:** End-to-end test a fresh personal deployment using **isolated Linux clients in simulated remote/mobile-like network paths**. Include NAT limitations and optional relay feasibility; do not claim real Android/iOS support before M7-04/M7-05.

**Acceptance:** clean install-to-connect documented and repeatable; missing permissions and failed connectivity produce actionable errors; secrets and protected routes survive reinstall/updates safely.

## M7 — consumer apps and operating-system integrations

- **M7-01:** Rust-first desktop application and OS integrations as **separate owner-approved platform subtasks** for Linux, Windows and macOS, with platform-specific privilege, firewall and DNS tests. No blanket cross-platform completion.
- **M7-02:** Secure edit/disconnect/reconnect flow, network change handling, status accuracy and UI close-to-tray/background behavior.
- **M7-03:** Minimum viewport/resizing/accessibility tests plus signed packages, updates and uninstall recovery.
- **M7-04:** Android/iOS VPN service integrations from shared Rust security core; document any unavoidable native language/OS framework glue and mobile VPN coexistence limits.
- **M7-05:** Test battery, roaming, app lifecycle, connection migration and failure diagnostics on physical devices.

**Acceptance:** a platform is listed as supported **only** after its complete T2/T3 tests pass. No blanket “100% Rust everywhere” claim without audited dependency inventory.

## M8 — independent release security work (G3)

- **M8-01:** Commission protocol and source audit, prioritizing handshake, nonce uniqueness, native integrations, privilege boundaries and update chain.
- **M8-02:** Reproduce/triage findings, fix security defects, regression/fuzz retest and have fixes independently reviewed where required.
- **M8-03:** Verify signed builds, reproducible release process, SBOM, license audit, disclosure policy and incident/patch workflow.
- **M8-04:** Publish honest limitations and platform support; owner decides G3.

**Acceptance:** independent review and remediation documented, every release-blocking T-test green, reproducible artifacts and explicit owner sign-off. **No “secure,” “audited,” “PQ-proof,” or public production release claims before G3.**

## M9 — enterprise/provider architecture (G4)

- **M9-01:** Design optional control plane and per-tenant identity/policy, with explicitly bounded stale credential lifetime.
- **M9-02:** Implement multi-node provisioning, APIs, SSO/MFA, tenant and admin access boundaries in separate reviewed tasks.
- **M9-03:** Implement gateway capacity selection, draining/failover, regional clusters, bounded handshake rates and DoS resilience.
- **M9-04:** Test data-plane continuity during coordination loss, mass revocation, overlapping tenant CIDRs and cross-tenant attacks.
- **M9-05:** Benchmark concurrent idle sessions **and active throughput** separately (pps, Gbps, P95/P99 latency, CPU, memory); define safe operational limits and owner G4 sign-off.

**Acceptance:** T4/T5 repeatable evidence, no tenant leaks, bounded failover/revocation; only measured performance/scale claims. **STOP until G4.**

## Execution sequence and discipline

**One owner-approved agent work order at a time, regardless of agent or tool.** Use the 43-card [TASK_CATALOG.md](TASK_CATALOG.md); each task must name files, expected behavior, non-goals, test commands, known risks, rollback and its exact stop point. Branch + PR for every change, code or documentation; owner reviews before merge and explicitly selects the next item. Re-check prior task state at each handoff. Agents must never infer completion from a timeout, create their own security decisions, publish an experimental VPN publicly, or skip to a “fun” feature.

**Current next action:** inspect owner-maintained [STATUS.md](STATUS.md), then M0-01 audit → M0-01.1 corrections if findings exist → M0-02 decisions → M0-03 G0 evidence, each separately owner-approved and accepted. Only then may the owner record G0 passage. **No cryptographic/protocol implementation before G1.**
