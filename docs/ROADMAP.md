# Septuvo — owner-controlled engineering roadmap

**Status: initial blueprint. Not an authorization to code or a schedule.** Follow [AGENTS.md](../AGENTS.md), [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md), [TASK_CATALOG.md](TASK_CATALOG.md), [TRACEABILITY.md](TRACEABILITY.md) and the [work-order template](WORK_ORDER_TEMPLATE.md). This file defines stages; the task catalog defines **exact per-task inputs, outputs and verification**. If any documents conflict, stop and record the ambiguity—do not choose independently. Milestones advance **in the order below**, except that track N1 runs in parallel after G0 (D-029). Under the lean process (D-030) the owner may approve a whole milestone with one comment; agents still finish one task at a time and stop at the end of each approved work order. Owner must explicitly approve every gate and work order. An agent must never continue to the next milestone because time remains.

## Gate summary

| Gate | Meaning | Authoritative state |
| --- | --- | --- |
| G0 | Owner approves scope, decisions, non-goals and this roadmap | [STATUS.md](STATUS.md) |
| G1 | Outside specialist reviews exact SVP design; owner approves prototype implementation | [STATUS.md](STATUS.md) |
| G2 | Lab interop, negative cases, no leaks and controlled faults; **not external pilot permission** | [STATUS.md](STATUS.md) |
| G3 | Independent security audit, remediation, signed release builds, platform validation | [STATUS.md](STATUS.md) |
| G4 | Provider isolation, availability, measured capacity and operational controls | [STATUS.md](STATUS.md) |

Only the owner's exact `GATE <Gn> PASSED` comment, recorded in the ledger, can pass a gate (D-031, category C). All gates are initially PENDING.

## Lean execution (D-030, simplified by D-031)

- **One authorization per milestone, in ordinary words** (D-031). An owner comment such as "go ahead with M2", or the owner's merge of the pull request that adds the milestone's work order, authorizes every task in it. **The owner's merge of each delivering pull request accepts that work**; no acceptance comments. Routine H work needs no authorization at all.
- **Work orders as files.** A work order is a file under `docs/work-orders/` in a pull request. The authorized revision is that file as merged, or as it stood when the owner's authorizing comment was posted.
- **Batched pull requests.** Inside an authorized milestone, an agent may deliver several tasks in one pull request (one commit per task, subject `type(<ID>): ...`), still finishing them one at a time and in order.
- **Two tracks after G0.** Track S: M1 requirements, then the M2 SVP specification. Track N1: the non-crypto Rust foundation below. Each track needs its own milestone authorization.
- **Unchanged:** no cryptographic or protocol implementation before G1 and a qualified outside review; every gate needs an owner `GATE` comment; every change goes through a pull request that the owner merges.

## N1 — non-crypto Rust foundation (parallel track, after G0, before G1; lab only)

- **N1-01:** Rust workspace skeleton, pinned toolchain and CI (format, lint, tests); no cryptography crates.
- **N1-02:** Route and LAN-exception policy model as pure, table-tested logic (IPv4/IPv6, overlapping prefixes, fail-closed default); no operating-system changes.
- **N1-03:** Linux lab harness in disposable network namespaces: apply and roll back route/firewall/DNS kill-switch policy and prove protected traffic is blocked when no VPN path exists. No tunnel and no traffic carrying.
- **N1-04:** CLI scaffolding and the four-stage connection-status model (adapter, authenticated peer, reachability, DNS — D-020), without networking.

**Acceptance:** builds and tests pass in CI and in the lab; nothing in N1 implements SVP, encryption, key handling or a data tunnel. N1 output feeds M5-01 and M5-02 later.

## N2 — non-crypto application foundation (parallel track, after G0, before G1)

Work order: [work-orders/N2.md](work-orders/N2.md).
- **N2-01:** configuration format, parser and validation (device and server).
- **N2-02:** privileged service `septuvod` and unprivileged control interface.
- **N2-03:** DNS policy enforcement and namespace leak tests.
- **N2-04:** server administration foundations (device records), with no enrollment, keys or invitations.

**Acceptance:** tested Rust code in CI and the namespace lab; placeholders only where the protocol will connect; nothing implements SVP, encryption, key handling, invitations or a data tunnel.

## N3 — installable lab service, captive portals, fuzzing, live status (parallel track, after G0, before G1)

Work order: [work-orders/N3.md](work-orders/N3.md).
- **N3-01:** installer and hardened systemd service setup (still lab-only), with a complete uninstall.
- **N3-02:** captive-portal mode (Q1): user-started, at most 5 minutes, kernel-enforced expiry, **for a dedicated helper account only**, so ordinary programs stay protected. Trusted DNS by default; the network's DNS only on explicit request, for the helper. RFC 8910/8908 discovery and a DoH client are planned for phase 2.
- **N3-03:** fuzz testing of every parser (standard library only).
- **N3-04:** live status view, `septuvo watch`.

**Acceptance:** as N2, plus a verified install/uninstall cycle on a disposable runner and recorded fuzz runs.

## N4 — Septuvo VPN desktop application for Linux (parallel track, after G0, before G1)

Work order: [work-orders/N4.md](work-orders/N4.md). An early, lab-only Linux slice of M7-01/M7-02; M7 itself is unchanged.
- **N4-01:** toolkit dependency review (GTK 4 + libadwaita via gtk-rs, `ksni` tray), application skeleton in a separate `desktop/` workspace, CI.
- **N4-02:** status and monitoring window (shared status model, service-lost banner, responsive and accessible).
- **N4-03:** tray icon and desktop notifications; closing is not disconnecting (D-019).
- **N4-04:** connect, disconnect and captive-portal controls through the control socket; permission guidance, no privilege prompts.
- **N4-05:** configuration management: service-mediated `CONFIG-GET` / `CONFIG-APPLY`, validated twice, atomic and fail-closed.
- **N4-06:** diagnostics: service `DIAGNOSE`, `septuvo diagnose`, app page, a local text report (no telemetry).
- **N4-07:** desktop packaging, accessibility and responsiveness pass, user guide.

**Acceptance:** CI green with headless UI tests and namespace lab cases for each task; never "connected" before G1; the main workspace stays free of third-party crates.

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

**One owner-approved agent work order at a time per track, regardless of agent or tool.** Use the 43-card [TASK_CATALOG.md](TASK_CATALOG.md); each task must name files, expected behavior, non-goals, test commands, known risks, rollback and its exact stop point. Branch + PR for every change, code or documentation; owner reviews before merge and explicitly selects the next item. Re-check prior task state at each handoff. Agents must never infer completion from a timeout, create their own security decisions, publish an experimental VPN publicly, or skip to a “fun” feature.

**Current next action:** inspect [STATUS.md](STATUS.md), kept in sync with merged pull requests by `scripts/sync_status.py`. G0 has passed; M1 and N1-01/N1-02 are merged. Next: the owner's answers to the M1 product questions ([M1-owner-questions.md](reviews/M1-owner-questions.md)), N1-03 and N1-04, and a work order for M2 (the SVP specification) for the owner to authorize → reviewer search and M2-05 → G1. **No cryptographic/protocol implementation before G1.**
