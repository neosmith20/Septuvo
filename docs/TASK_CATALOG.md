# Septuvo — task-by-task execution catalog

**Planning blueprint, not an authorization to code.** Read [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md), [AGENTS.md](../AGENTS.md), [ROADMAP.md](ROADMAP.md), [SECURITY_MODEL.md](SECURITY_MODEL.md) and the [work-order template](WORK_ORDER_TEMPLATE.md) before using any task card.

**Rules applying to every card:** Consult [STATUS.md](STATUS.md), kept in sync with GitHub (D-031). The owner authorizes each milestone's work order in ordinary words; one authorization may cover one card or a whole milestone (D-030). The immediately preceding task must be complete, meaning delivered by a pull request the owner merged; inside an authorized milestone, tasks run in order. Named output paths are intended deliverables, **not an unrestricted permission to create files**. If they need changing, get a written scope update. The current gate must have recorded evidence. Commit only scoped files on a task branch, open the pull request yourself with test logs, and **STOP** for review; after the owner merges it, continue with the next task of the same milestone (D-034). Never infer that describing a later milestone authorizes starting it.

## Fixed task order

**M0:** M0-01 → (M0-01.1 corrections when audit finds defects) → M0-02 → M0-03

**Track N1 (parallel, after G0, before G1):** N1-01 → N1-02 → N1-03 → N1-04

**M1:** M1-01 → M1-02 → M1-03 → M1-04

**M2:** M2-01 → M2-02 → M2-03 → M2-04 → M2-05

**M3:** M3-01 → M3-02 → M3-03 → M3-04

**M4:** M4-01 → M4-02 → M4-03 → M4-04

**M5:** M5-01 → M5-02 → M5-03 → M5-04 → M5-05

**M6:** M6-01 → M6-02 → M6-03 → M6-04

**M7:** M7-01 → M7-02 → M7-03 → M7-04 → M7-05

**M8:** M8-01 → M8-02 → M8-03 → M8-04

**M9:** M9-01 → M9-02 → M9-03 → M9-04 → M9-05

## Standard per-task completion evidence

Use the one mandatory [WORK_ORDER_TEMPLATE.md](WORK_ORDER_TEMPLATE.md) handoff form. Every task remains IN REVIEW until the owner merges the pull request that delivers it (D-031). Base cards (singly or per milestone), N1 cards and dotted subtasks need an owner-authorized work order. Routine H maintenance needs none.

## M0 — Repository baseline and owner approval

### M0-01 — Audit existing planning documents

- **Entry / prerequisites:** Owner-authorized documentation audit; G0 not yet required.
- **Approved output locations:** `docs/reviews/M0-01-audit.md`.
- **Exact work:** Compare README, license, AGENTS.md, SECURITY.md, roadmap, task catalog, STATUS ledger, agent operations handbook, work-order template, architecture, security model (threat model), decision register, traceability, test matrix and protocol checklist; list every mismatch with path/heading, severity and proposed correction; do not reinterpret a candidate as approved.
- **Acceptance evidence:** Report all cross-file naming, gates, crypto-state, relative links, requirements and task IDs; record unresolved findings as OPEN. No source or protocol edits.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M0-02 until this pull request is merged; then continue without a new authorization (D-034).

### M0-01.1 — Correct M0-01 audit defects (conditional)

- **Entry / prerequisites:** M0-01 accepted; defects identified; explicit owner-approved subtask in STATUS.md.
- **Approved output locations:** Exact affected planning documents in the subtask work order; docs/reviews/M0-01-audit.md finding status only if approved.
- **Exact work:** Apply only the defects identified by M0-01, with links to findings and bounded changes. No networking, crypto, live infrastructure, brand asset or unrelated modification.
- **Acceptance evidence:** Each finding resolved or explicitly left OPEN; documentation CI and actual test commands attached.
- **Stop point:** Submit scoped PR, wait for owner to mark M0-01.1 ACCEPTED. If no findings, owner records a waiver in M0-01 acceptance evidence before M0-02.

### M0-02 — Resolve baseline product decisions

- **Entry / prerequisites:** M0-01 accepted; if audit found defects, M0-01.1 owner-accepted; owner-approved decision session.
- **Approved output locations:** `docs/reviews/M0-02-owner-decisions.md`, `docs/DECISIONS.md`.
- **Exact work:** Present explicit yes/no choices for supported OS order, language exceptions, first lab platform, target user journeys, self-hosting and risk assumptions; record rationale and owner response.
- **Acceptance evidence:** Every requested decision has a source/approval or is OPEN; never fill blanks or approve cryptographic details.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M0-03 until this pull request is merged; then continue without a new authorization (D-034).

### M0-03 — Plan review resources, lab and change control

- **Entry / prerequisites:** M0-02 accepted.
- **Approved output locations:** `docs/reviews/G0.md`.
- **Exact work:** Prepare the G0 evidence package in one document: scope, decisions and their states, open questions, acceptance criteria for M1, M2 and N1, the lab plan for D-026, change control (D-023, D-025, the `main` ruleset) and the outside-reviewer plan (requirements and search plan; the search itself happens during M2, when there is a specification to review).
- **Acceptance evidence:** Owner decides G0 with `GATE G0 PASSED` or `GATE G0 FAILED`; missing items are listed as OPEN rather than silently assumed.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M1 — Requirements and threat modeling

### M1-01 — Map trust boundaries and attackers

- **Entry / prerequisites:** G0 passed; M0-03 accepted.
- **Approved output locations:** `docs/spec/threats.md`, `docs/SECURITY_MODEL.md`.
- **Exact work:** Describe assets, device/server/control-plane trust, hostile LAN/on-path attackers, endpoint compromise, recorded traffic, malicious tenants, update supply chain and realistic non-goals.
- **Acceptance evidence:** Every S-01..S-13 security requirement cites at least one adversary and assumption; owner reviews newly exposed risks.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M1-02 until this pull request is merged; then continue without a new authorization (D-034).

### M1-02 — Define network and user journeys

- **Entry / prerequisites:** M1-01 accepted.
- **Approved output locations:** `docs/spec/network-policies.md`.
- **Exact work:** Write step-by-step home NAS direct access, away-from-home NAS access, VPN exit, explicit LAN exceptions, subnet overlap, IPv6/DNS behavior, routes, firewall and fail-closed failures.
- **Acceptance evidence:** Each scenario declares packet path, expected DNS path, exception source, authentication and leakage failure; no claims that routing alone acts as kill switch.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M1-03 until this pull request is merged; then continue without a new authorization (D-034).

### M1-03 — Design enrollment, revocation and privacy requirements

- **Entry / prerequisites:** M1-02 accepted.
- **Approved output locations:** `docs/spec/identity-and-privacy.md`.
- **Exact work:** Document pinning/enrollment choices, single-use token semantics, key storage/recovery, revocation, policy expiry, local-only operation and telemetry retention/consent requirements.
- **Acceptance evidence:** No invitation carries private keys; missing platform key-store capabilities and revocation bounds are called out explicitly.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M1-04 until this pull request is merged; then continue without a new authorization (D-034).

### M1-04 — Map requirements to tests and platforms

- **Entry / prerequisites:** M1-03 accepted.
- **Approved output locations:** `docs/TRACEABILITY.md`, `docs/spec/platform-feasibility.md`.
- **Exact work:** Map S-01..S-13 and product requirements to owning component, test IDs, release gate and supported/unknown OS behavior; identify dependencies and exclusions.
- **Acceptance evidence:** No orphan security requirement or unsupported blanket platform claim; owner accepts M1 baseline.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M2 — SVP specification and external G1 review

### M2-01 — Study existing constructions and Rust crypto candidates

- **Entry / prerequisites:** M1-04 accepted; G0 passed.
- **Approved output locations:** `docs/protocol/candidate-analysis.md`.
- **Exact work:** Compare reviewed key exchanges, identity constructions, KEM/signature libraries, AEAD, latency/packet-size costs, licenses, unsafe/FFI use and maintenance; link standards and independent assessments.
- **Acceptance evidence:** Give at least two viable approaches and explicit tradeoffs; preferred AES-256-GCM and hybrid KEM remain candidates until decision; no crypto code.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M2-02 until this pull request is merged; then continue without a new authorization (D-034).

### M2-02 — Draft identity and handshake specification

- **Entry / prerequisites:** M2-01 accepted.
- **Approved output locations:** `docs/protocol/handshake.md`, `docs/protocol/state-machine.md`.
- **Exact work:** Specify peer roles, trust roots, serialized transcript, negotiation binding, KEM combiner, KDF steps, key confirmation, authentication, timeout/error states and downgrade handling for specialist review.
- **Acceptance evidence:** Every state transition and cryptographic input is explicit; unresolved constructions labeled BLOCKED, not invented or borrowed without review.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M2-03 until this pull request is merged; then continue without a new authorization (D-034).

### M2-03 — Draft transport, replay and key-lifecycle specification

- **Entry / prerequisites:** M2-02 accepted.
- **Approved output locations:** `docs/protocol/wire-format.md`, `docs/protocol/key-lifecycle.md`.
- **Exact work:** Specify byte-level parser grammar, AEAD authenticated fields and nonce invariants, packet numbers, anti-replay, epoch rollover, MTU/handshake fragmentation, amplification limits, roaming and crash recovery.
- **Acceptance evidence:** Independent implementer could interpret each field unambiguously; collision/replay/downgrade/error cases documented; no implementation.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M2-04 until this pull request is merged; then continue without a new authorization (D-034).

### M2-04 — Prepare spec conformance and test-vector proposal

- **Entry / prerequisites:** M2-03 accepted; internal draft reviewed; external G1 still pending.
- **Approved output locations:** `docs/protocol/test-vectors.md`, `docs/protocol/conformance-plan.md`.
- **Exact work:** Documentation only: propose wire-fixture formats, negative cases, state/replay scenarios, future property-test requirements and independent implementation conformance methodology. **Do not compute test vectors or write crypto/protocol generators, property-test code, or live-network code before G1.** Per D-021 (owner decision: option C), computed vectors are deferred until after G1.
- **Acceptance evidence:** Every P-01..P-14 item maps to a section or explicitly OPEN finding. Computed vectors are recorded as NOT RUN (deferred to after G1 by D-021 option C); the plan states how they will be produced and checked after G1 and is submitted for G1 reviewer acceptance; no live-network code.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M2-05 until this pull request is merged; then continue without a new authorization (D-034).

### M2-05 — Obtain external protocol assessment and G1 decision

- **Entry / prerequisites:** M2-04 accepted; qualified external reviewer identified (the agent helps the owner search, including free or low-cost options).
- **Approved output locations:** `docs/reviews/G1.md`, `reviewer report/artifacts as permitted`, `docs/DECISIONS.md`.
- **Exact work:** Arrange qualified independent assessment of exact specification revision, respond to findings in small reviewed revisions, and assemble evidence for owner approval.
- **Acceptance evidence:** Reviewer identity/qualification, exact reviewed commit, findings, residual risk and owner decision recorded; no agent may self-certify G1; remain BLOCKED if review unavailable.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M3 — Reviewed Rust core in lab

### M3-01 — Create minimal Rust workspace and quality baseline

- **Entry / prerequisites:** M2-05 accepted; G1 passed; owner approves source skeleton.
- **Approved output locations:** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/`, `.github/workflows/`, `docs/build/`.
- **Exact work:** Propose and approve exact crate boundaries; add minimal compilable Rust skeleton, license metadata, pinned compiler policy, fmt/clippy/test CI, dependency/license/secret scanning as approved.
- **Acceptance evidence:** Clean bootstrap on supported Linux CI; fmt/clippy/tests actually run; no transports, handshakes, VPN drivers or unnecessary crates.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M3-02 until this pull request is merged; then continue without a new authorization (D-034).

### M3-02 — Implement bounded wire parsing

- **Entry / prerequisites:** M3-01 accepted and reviewed wire grammar approved at G1.
- **Approved output locations:** `crates/svp-wire/`, `tests/wire/`.
- **Exact work:** Implement parsing/serialization of only reviewed SVP fields; limit lengths/allocations; reject unknown versions/invalid sizes; fuzz parsing without privileges.
- **Acceptance evidence:** Reviewed canonical vectors, truncation, overlong frames and fuzz cases pass; no panics; no reinterpretation of fields.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M3-03 until this pull request is merged; then continue without a new authorization (D-034).

### M3-03 — Integrate approved cryptographic primitives

- **Entry / prerequisites:** M3-02 accepted; exact dependencies owner-approved; G1 construction frozen.
- **Approved output locations:** `crates/svp-crypto/`, `docs/build/crypto-dependencies.md`, `tests/crypto/`.
- **Exact work:** Use reviewed Rust libraries and exact protocol KDF/signature/KEM/AEAD schedule; protect secret memory as documented; isolate random generation and per-direction keys.
- **Acceptance evidence:** Known-answer vectors and negative key/signature/tag tests pass; dependency inventory and exception report complete; never claim primitive certification equals VPN certification.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M3-04 until this pull request is merged; then continue without a new authorization (D-034).

### M3-04 — Implement reviewed session state machine

- **Entry / prerequisites:** M3-03 accepted.
- **Approved output locations:** `crates/svp-session/`, `tests/session/`.
- **Exact work:** Implement only reviewed handshake, timeout/retry, peer authentication, counters, epochs and replay handling; inject packet loss, reorder, duplicate, disconnect and key rotation.
- **Acceptance evidence:** Concurrency, duplicate/replay, downgrade, crash-restart and nonce-uniqueness tests pass; no real network or public listener.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M4 — Isolated Linux tunnel

### M4-01 — Build isolated TUN/UDP transport plumbing

- **Entry / prerequisites:** M3-04 accepted; isolated lab explicitly authorized.
- **Approved output locations:** `crates/septuvo-node/`, `crates/septuvo-engine/`, `tests/lab/transport/`.
- **Exact work:** In approved Linux network namespaces/VMs, connect reviewed session engine to TUN and UDP using least privilege; provide cleanup scripts and failure recovery.
- **Acceptance evidence:** Namespace-only fixtures can start/stop without changing live/default host routes or firewall; verify privilege boundaries and cleanup after forced crash.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M4-02 until this pull request is merged; then continue without a new authorization (D-034).

### M4-02 — Carry encrypted IPv4 and IPv6 in lab

- **Entry / prerequisites:** M4-01 accepted.
- **Approved output locations:** `crates/septuvo-engine/`, `tests/lab/ipv4-ipv6/`.
- **Exact work:** Tunnel bounded IPv4 and IPv6 packets between mutually authenticated lab nodes with negotiated reviewed sessions; enforce gateway destination permission.
- **Acceptance evidence:** Traffic capture confirms no IP plaintext on UDP underlay; positive and unauthorized/invalid peer cases documented.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M4-03 until this pull request is merged; then continue without a new authorization (D-034).

### M4-03 — Verify failure, rekey, loss and recovery

- **Entry / prerequisites:** M4-02 accepted.
- **Approved output locations:** `tests/lab/faults/`, `docs/reviews/M4-fault-evidence.md`.
- **Exact work:** Exercise lost/reordered packets, MTU issues, key rollover, power/process restart, invalid tags, fake peers, session collisions and fallback behavior.
- **Acceptance evidence:** No key/nonce reuse, auth bypass or plaintext leak; failures produce bounded errors and resource usage; identify all untested cases.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M4-04 until this pull request is merged; then continue without a new authorization (D-034).

### M4-04 — Record honest lab performance baseline

- **Entry / prerequisites:** M4-03 accepted.
- **Approved output locations:** `bench/`, `docs/reviews/M4-benchmarks.md`.
- **Exact work:** Specify controlled topology; measure baseline/Septuvo throughput, packet rate, latency percentiles, CPU and RAM across packet sizes and acceleration settings.
- **Acceptance evidence:** Publish exact hosts/OS/tool versions/commands and raw results; measurements labeled LAB and cannot substantiate marketing claims.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M5 — Routing, fail-closed enforcement and G2

### M5-01 — Implement route and LAN-policy model

- **Entry / prerequisites:** M4-04 accepted; M1 policies approved; builds on N1-02 where accepted.
- **Approved output locations:** `crates/septuvo-policy/`, `tests/policy/`.
- **Exact work:** Separate route selection from authorization; model strict/approved-LAN/custom modes, exclusions, overlapping prefixes, remote home subnets and IPv4/IPv6 paths.
- **Acceptance evidence:** Table-driven route tests cover local NAS, remote NAS, exit path and conflicts; no default RFC1918 trust or silent cleartext route.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M5-02 until this pull request is merged; then continue without a new authorization (D-034).

### M5-02 — Implement OS-enforced kill switch and DNS policy

- **Entry / prerequisites:** M5-01 accepted; dedicated lab permissions granted; builds on N1-03 where accepted.
- **Approved output locations:** `crates/septuvo-policy/`, `tests/lab/leaks/`, `docs/build/linux-policy.md`.
- **Exact work:** Apply atomic/recoverable Linux route/firewall/DNS transitions with explicit underlay exceptions; test reconnect, update, crash and VPN loss under hostile network.
- **Acceptance evidence:** Captured proof protected IPv4, IPv6 and DNS do not leak after failures; exceptions are narrowly approved; host routing can be restored.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M5-03 until this pull request is merged; then continue without a new authorization (D-034).

### M5-03 — Verify home and remote access flows

- **Entry / prerequisites:** M5-02 accepted.
- **Approved output locations:** `tests/lab/home-gateway/`, `docs/reviews/M5-user-journeys.md`.
- **Exact work:** Model one directly reachable home NAS and one remote home subnet, plus optional separate exit; exercise LAN-allowed vs strict, same-CIDR conflicts and DNS reachability.
- **Acceptance evidence:** Observed paths match M1 requirements, no cross-policy bypass and no blanket untrusted LAN access; cases that OS cannot support reported.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M5-04 until this pull request is merged; then continue without a new authorization (D-034).

### M5-04 — Fault-injection and soak lab qualification

- **Entry / prerequisites:** M5-03 accepted.
- **Approved output locations:** `tests/lab/resilience/`, `docs/reviews/M5-resilience.md`.
- **Exact work:** Run packet mutation, loss/reorder, **roaming (where available), fragmentation/MTU failures**, fuzz corpora, bounded-resource DoS simulations, IPv6/DNS leak checks, privilege/replay tests, memory/concurrency checks and extended soak tests within lab quotas.
- **Acceptance evidence:** T1/T2 evidence with exact environments and quantitative observations; failed tests block progress; no live external load tests.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M5-05 until this pull request is merged; then continue without a new authorization (D-034).

### M5-05 — Assemble G2 lab gate decision

- **Entry / prerequisites:** M5-04 accepted.
- **Approved output locations:** `docs/reviews/G2.md`, `docs/TEST_MATRIX.md`.
- **Exact work:** Collect reproducible interop, negative, leak, recovery, known-limit and privacy results; request owner decision on the precisely tested lab revision.
- **Acceptance evidence:** G2 recorded by the owner as PASSED or FAILED, open issues triaged; a successful lab tunnel does NOT authorize public production.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M6 — Standalone server and guided first-use

### M6-01 — Create local-only onboarding, identity and revocation

- **Entry / prerequisites:** G2 passed; M5-05 accepted.
- **Approved output locations:** `crates/septuvo-node/`, `docs/spec/enrollment-user-flows.md`, `tests/enrollment/`.
- **Exact work:** Build secure device enrollment and credential storage according to approved identity spec; implement expiration/revocation and zero-vendor-account standalone mode.
- **Acceptance evidence:** Owner can onboard/revoke from isolated lab; stolen/expired invitation denied; no private keys in QR or logs; documented offline behavior.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M6-02 until this pull request is merged; then continue without a new authorization (D-034).

### M6-02 — Package self-hosted Linux server safely

- **Entry / prerequisites:** M6-01 accepted.
- **Approved output locations:** `packaging/linux/`, `docs/install/linux-server.md`, `tests/install/`.
- **Exact work:** Implement lab-installable **unsigned pre-release** Linux server packaging with automated install/uninstall, service supervision, least-privilege defaults, port/firewall guidance and rollback. Document signing plan; actual production release signing belongs to M8-03.
- **Acceptance evidence:** Fresh lab install/uninstall/reboot tested; no unexpected public exposure, route leftovers or unapproved package installs.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M6-03 until this pull request is merged; then continue without a new authorization (D-034).

### M6-03 — Implement understandable setup and diagnostics

- **Entry / prerequisites:** M6-02 accepted.
- **Approved output locations:** `crates/septuvo-cli/`, `apps/server-setup/`, `tests/diagnostics/`.
- **Exact work:** Guide hosting and joining without compiling crates; expose distinct adapter, authenticated handshake, tunnel reachability and DNS status with useful failures.
- **Acceptance evidence:** Novice script: install→create→enroll→connect→diagnose; wrong endpoint/key/firewall/DNS tested and accurately described.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M6-04 until this pull request is merged; then continue without a new authorization (D-034).

### M6-04 — Perform personal-use acceptance scenario

- **Entry / prerequisites:** M6-03 accepted.
- **Approved output locations:** `tests/e2e/personal/`, `docs/reviews/M6-personal-acceptance.md`.
- **Exact work:** Test home Jellyfin-like service via isolated two-site network; explicitly model NAT/CGNAT restrictions and opt-in relay feasibility; do NOT imply phone support until M7 mobile is tested.
- **Acceptance evidence:** Third-party novice can follow packaged steps in supported Linux lab; capture real access and failure evidence; no cloud required.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M7 — User apps and OS-specific integration

### M7-01 — Plan and implement desktop platform adapters incrementally

- **Entry / prerequisites:** M6-04 accepted; individual OS work orders required.
- **Approved output locations:** `apps/desktop/`, `crates/platform/`, `docs/build/platform-matrix.md`.
- **Exact work:** Build minimal Rust-first UI/service boundary; integrate Linux then Windows/macOS only by separately owner-approved subtask, using OS VPN APIs and declared native glue.
- **Acceptance evidence:** Per-OS TUN, privilege, DNS/firewall, install and restart tests pass independently; no platform marked supported without evidence.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M7-02 until this pull request is merged; then continue without a new authorization (D-034).

### M7-02 — Implement safe edit, close and connection-state UX

- **Entry / prerequisites:** M7-01 accepted on an actually tested desktop target.
- **Approved output locations:** `apps/desktop/`, `tests/ux/`.
- **Exact work:** Model edit-while-connected with authenticated disconnect/rekey/reconnect or safe refusal; tray/quit behavior explicit; show status by tested handshake/data/DNS stages.
- **Acceptance evidence:** Editing does not expose plaintext or lose configuration; close-to-tray keeps intended tunnel; all failure states produce actionable messages.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M7-03 until this pull request is merged; then continue without a new authorization (D-034).

### M7-03 — Validate desktop packaging and accessibility

- **Entry / prerequisites:** M7-02 accepted.
- **Approved output locations:** `packaging/desktop/`, `tests/install/`, `docs/reviews/M7-desktop.md`.
- **Exact work:** Test UI on 1920x1080 and smaller windows; resizing/scrolling, keyboard access, updates, signed installers and uninstall; run security/leak regressions.
- **Acceptance evidence:** Per-platform clean install/update/reboot/uninstall evidence; no inaccessible content and no route/firewall leftovers.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M7-04 until this pull request is merged; then continue without a new authorization (D-034).

### M7-04 — Integrate Android and iOS as separate reviewed platforms

- **Entry / prerequisites:** M7-03 accepted; mobile API feasibility and code inventory reviewed.
- **Approved output locations:** `apps/mobile/`, `crates/platform/`, `docs/build/platform-matrix.md`.
- **Exact work:** Reuse Rust security engine; use OS-required VPN extension/service bindings; record any Swift/Kotlin/native glue and OS single-VPN/background limitations.
- **Acceptance evidence:** Real device per platform authenticates and carries encrypted IPv4/IPv6; cryptographic code stays Rust; platform exclusions and exceptions documented.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M7-05 until this pull request is merged; then continue without a new authorization (D-034).

### M7-05 — Validate real mobile lifecycle and connection safety

- **Entry / prerequisites:** M7-04 accepted only for target platforms actually implemented.
- **Approved output locations:** `tests/mobile/`, `docs/reviews/M7-mobile.md`.
- **Exact work:** Test Wi-Fi↔cellular switches, app sleep/wake, tunnel persistence, LTE/5G NAT cases, battery/thermal effects, device revoke, DNS and kill-switch leaks.
- **Acceptance evidence:** Real-device test reports with OS versions and logs; each platform recorded separately as supported or unsupported in the platform matrix, never blanket claims.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M8 — Independent audit and controlled production release

### M8-01 — Commission independent security audit

- **Entry / prerequisites:** M7-05 accepted; lab and product evidence ready; owner approves external engagement.
- **Approved output locations:** `docs/reviews/G3-audit-scope.md`, `confidential reviewer report reference`.
- **Exact work:** Supply exact protocol and implementation revision, build reproducibility and test corpus to qualified independent reviewers; include OS policy, update chain, crypto and privileged boundaries.
- **Acceptance evidence:** Auditor identity/scope/date/commit identified; findings classified; no agent-generated report represented as independent audit.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M8-02 until this pull request is merged; then continue without a new authorization (D-034).

### M8-02 — Remediate and re-review audit findings

- **Entry / prerequisites:** M8-01 audit report received and owner prioritizes findings.
- **Approved output locations:** `docs/reviews/G3-remediation.md`, `approved scoped code PRs`, `regression tests`.
- **Exact work:** Address findings one owner-approved task/PR at a time, add reproducer and regression evidence, and send crypto-affecting changes back for required external review.
- **Acceptance evidence:** No high/critical unresolved exploitable finding hidden; revised commit examined as needed; tests real, and change scope narrow.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M8-03 until this pull request is merged; then continue without a new authorization (D-034).

### M8-03 — Prepare signed, recoverable release process

- **Entry / prerequisites:** M8-02 accepted.
- **Approved output locations:** `docs/release/`, `packaging/`, `.github/workflows/`.
- **Exact work:** Establish SBOM and dependency/license inventory, sign/check installer provenance, update channels, rollback/revocation, security contact and incident runbooks.
- **Acceptance evidence:** Fresh and upgrade installations verified on every claimed OS; release artifact signatures verified; signing keys not committed.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M8-04 until this pull request is merged; then continue without a new authorization (D-034).

### M8-04 — Request G3 owner release decision

- **Entry / prerequisites:** M8-03 accepted.
- **Approved output locations:** `docs/reviews/G3.md`, `docs/release/support-matrix.md`.
- **Exact work:** Consolidate independent audit, remediation, signed binaries, test matrix, documented limits and product claims; propose narrowly scoped supported release.
- **Acceptance evidence:** Owner explicitly approves release and version; untested platforms/capabilities remain excluded; no unaudited public security claims.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.


## M9 — Enterprise/provider readiness and G4

### M9-01 — Design multi-tenant control plane and limited-stale policy

- **Entry / prerequisites:** M8-04 accepted; G3 passed for applicable core; enterprise scope owner-approved.
- **Approved output locations:** `docs/enterprise/control-plane.md`, `docs/enterprise/tenant-model.md`.
- **Exact work:** Specify data-plane/control-plane boundaries, tenant identity/policy/IP overlap, bounded revocation, outage caching, secret storage and HA assumptions.
- **Acceptance evidence:** Cross-tenant access denied by design, trust rules and revocation latencies explicit; owner approves architecture prior to implementation.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M9-02 until this pull request is merged; then continue without a new authorization (D-034).

### M9-02 — Implement enterprise management in scoped slices

- **Entry / prerequisites:** M9-01 accepted; each feature separate owner-approved subtask.
- **Approved output locations:** `crates/septuvo-control/`, `tests/enterprise/authz/`, `docs/enterprise/`.
- **Exact work:** Implement provisioning and API auth first; then optional SSO/MFA and RBAC; test every role/tenant boundary with positive and negative fixtures.
- **Acceptance evidence:** No unauthenticated management operation, missing authorization boundary or cross-tenant leakage; exact supported features documented.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M9-03 until this pull request is merged; then continue without a new authorization (D-034).

### M9-03 — Build HA and gateway placement controls

- **Entry / prerequisites:** M9-02 accepted.
- **Approved output locations:** `crates/septuvo-control/`, `tests/enterprise/ha/`, `docs/enterprise/operations.md`.
- **Exact work:** Add rate-limited enrollment/handshake control, gateway capacity selection, draining, cluster recovery and authenticated session/state migration if reviewed.
- **Acceptance evidence:** Bounded loss, memory, CPU and outage behavior verified; no key/nonce reuse on migration; control plane cannot silently weaken data-plane policy.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M9-04 until this pull request is merged; then continue without a new authorization (D-034).

### M9-04 — Validate isolation under failure and abuse

- **Entry / prerequisites:** M9-03 accepted.
- **Approved output locations:** `tests/enterprise/isolation/`, `docs/reviews/M9-isolation.md`.
- **Exact work:** Exercise overlapping tenant subnets, stolen/revoked credentials, malicious tenant packet floods in lab, control-plane outages and gateway failures.
- **Acceptance evidence:** No cross-tenant unauthorized packets, policy bypass or revoked-user acceptance beyond approved bounds; reproducible incidents recorded.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M9-05 until this pull request is merged; then continue without a new authorization (D-034).

### M9-05 — Benchmark scale and seek G4 approval

- **Entry / prerequisites:** M9-04 accepted.
- **Approved output locations:** `bench/provider/`, `docs/reviews/G4.md`, `docs/enterprise/capacity.md`.
- **Exact work:** Measure thousands of idle vs active connections separately, pps/Gbps, latency, CPU, memory, failure recovery, admission limits and geography assumptions; request owner G4 decision.
- **Acceptance evidence:** Owner approves only empirically demonstrated capacities and tested architecture; no extrapolating 1000 users into a million.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin the next milestone automatically.



## Track N1 — Non-crypto Rust foundation (parallel, after G0, before G1)

Lab-only work allowed by D-029 and D-030. **Never** an SVP handshake, packet format, key handling, encryption or a data tunnel, and no cryptography crates. One owner authorization covers the whole track (D-031).

### N1-01 — Rust workspace skeleton and CI

- **Entry / prerequisites:** G0 passed; approved N1 (or N1-01) work order.
- **Approved output locations:** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/septuvo-policy/`, `crates/septuvo-cli/`, `.github/workflows/rust.yml`, `docs/build/README.md`.
- **Exact work:** Minimal compilable workspace with two crates; pinned toolchain; CI running format, lint (warnings denied) and tests; GitHub Actions pinned to full commit SHAs with read-only permissions; no third-party crates unless the approved work order lists them.
- **Acceptance evidence:** CI green on GitHub; no network, cryptography or operating-system-changing code.
- **Stop point:** Submit the scoped evidence and PR for owner review.

### N1-02 — Route and LAN-exception policy model (pure logic)

- **Entry / prerequisites:** N1-01 complete within the approved N1 work order, or accepted.
- **Approved output locations:** `crates/septuvo-policy/`.
- **Exact work:** Data model and decision function for protected versus exempt destinations: full tunnel, explicitly approved LAN destinations, custom split prefixes, IPv4 and IPv6, overlapping prefixes, deterministic conflict resolution and a fail-closed default; no implicit trust of RFC1918 ranges (D-006).
- **Acceptance evidence:** Table-driven unit tests for the home-NAS, remote-NAS, exit-path and conflict cases of D-027; invalid input rejected; no operating-system calls.
- **Stop point:** Submit the scoped evidence and PR for owner review.

### N1-03 — Linux namespace lab harness for kill switch and DNS policy

- **Entry / prerequisites:** N1-02 complete within the approved N1 work order, or accepted.
- **Approved output locations:** `tests/lab/namespaces/`, `docs/build/lab.md`.
- **Exact work:** Scripts and a test harness that, inside disposable network namespaces on a lab VM or a GitHub-hosted runner, apply and roll back firewall/route/DNS rules generated from the N1-02 model and check that protected IPv4, IPv6 and DNS traffic is blocked when no VPN path exists while approved exceptions work. Root privileges only inside the disposable lab machine; no tunnel, no internet traffic, no change to any real host.
- **Acceptance evidence:** Reproducible leak tests pass; rollback leaves no rules behind; limitations documented.
- **Stop point:** Submit the scoped evidence and PR for owner review.

### N1-04 — CLI scaffolding and connection-status model

- **Entry / prerequisites:** N1-03 complete within the approved N1 work order, or accepted.
- **Approved output locations:** `crates/septuvo-cli/`.
- **Exact work:** Command structure (for example `status` and `policy check`) using the standard library only; a status model with four independent stages — adapter, authenticated peer, reachability, DNS (D-020) — represented as data; no networking.
- **Acceptance evidence:** Unit tests show that "adapter up" never implies "connected".
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin M3 or any cryptography.

## Track N2 — Non-crypto application foundation (parallel, after G0, before G1)

Work allowed by D-029, defined in [work-orders/N2.md](work-orders/N2.md). **Never** an SVP handshake, packet encryption or wire encoding, key generation or management, usable invitations, functional protocol transport or any cryptography crate. Interfaces for later protocol work have placeholder implementations only, which return errors and never report a peer as authenticated. One owner authorization covers the track (D-031); after each merge the agent continues to the next card (D-034).

### N2-01 — Configuration format, parser and validation

- **Entry / prerequisites:** N1 accepted; approved N2 work order.
- **Approved output locations:** `crates/septuvo-config/`, `docs/build/config.md`, `licensing.toml`, `Cargo.toml`, `Cargo.lock`, CLI wiring named in the work order.
- **Exact work:** Strict TOML-subset parser (standard library only) and validated device and server schemas, with hostile-input limits, precise error locations and safe defaults for security settings.
- **Acceptance evidence:** Table-driven tests for every rule; malformed files rejected without panics; CI green; licensing check passes.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N2-02 until this pull request is merged; then continue without a new authorization (D-034).

### N2-02 — Privileged service and unprivileged control interface

- **Entry / prerequisites:** N2-01 complete within the approved N2 work order.
- **Approved output locations:** `crates/septuvo-ipc/`, `crates/septuvo-daemon/`, `crates/septuvo-cli/`, `tests/lab/namespaces/`, `.github/workflows/rust.yml`, `docs/build/daemon.md`, `licensing.toml`, `Cargo.toml`, `Cargo.lock`.
- **Exact work:** `septuvod` owns all network changes; status and control Unix sockets with permission-based access; bounded, versioned local protocol; lab-mode and namespace guard; placeholder tunnel interface only.
- **Acceptance evidence:** Parser tests; namespace integration test (unprivileged status, refused control from non-members, kill switch applied and removed, crash keeps blocking); guard test.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N2-03 until this pull request is merged; then continue without a new authorization (D-034).

### N2-03 — DNS policy enforcement and leak tests

- **Entry / prerequisites:** N2-02 complete within the approved N2 work order.
- **Approved output locations:** `crates/septuvo-policy/`, `crates/septuvo-daemon/`, `tests/lab/namespaces/`, `docs/build/lab.md`.
- **Exact work:** DNS only to configured tunnel resolvers over the tunnel interface; UDP/TCP 53 and DoT blocked elsewhere for IPv4 and IPv6; resolver configuration written and restored; explicit LAN DNS exceptions; DoH limitation documented.
- **Acceptance evidence:** New namespace leak cases pass in CI with IPv6 required; all N1-03 cases still pass.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N2-04 until this pull request is merged; then continue without a new authorization (D-034).

### N2-04 — Server administration foundations

- **Entry / prerequisites:** N2-03 complete within the approved N2 work order.
- **Approved output locations:** `crates/septuvo-server/`, `docs/build/server-admin.md`, `licensing.toml`, `Cargo.toml`, `Cargo.lock`.
- **Exact work:** Local `septuvo-server` tool for device records (add, list, show, revoke, remove), address allocation, atomic fail-closed storage and an action log; placeholder key field; no invitations, enrollment, keys or listener.
- **Acceptance evidence:** Unit tests for commands, allocation, interrupted writes, corruption, locking and hostile names.
- **Stop point:** Submit the scoped evidence and PR for owner review. N2 is then complete; nothing beyond it starts without a new work order.

## Track N3 — Installable lab service, captive portals, fuzzing, live status (parallel, after G0, before G1)

Work allowed by D-029, defined in [work-orders/N3.md](work-orders/N3.md). Same boundary as N2: **never** an SVP handshake, packet encryption or wire encoding, key generation or management, usable invitations, functional protocol transport or any cryptography crate; the service stays lab-only. One owner authorization covers the track (D-031); after each merge the agent continues to the next card (D-034).

### N3-01 — Installer and service setup (Linux, lab mode)

- **Entry / prerequisites:** N2 accepted; approved N3 work order.
- **Approved output locations:** `packaging/linux/`, `tests/lab/install/`, `.github/workflows/rust.yml`, `docs/build/install.md`, `LICENSING.md`.
- **Exact work:** Hardened systemd units (lab namespace unit, service joining it), sysusers and tmpfiles files, configuration templates, idempotent `install.sh` with `DESTDIR`, complete `uninstall.sh` (disconnect, remove rules, units, files, group; keep configuration unless `--purge`).
- **Acceptance evidence:** `systemd-analyze verify` passes; `DESTDIR` file list, modes and owners exact; real install on a disposable CI runner works end to end; uninstall leaves nothing; re-install changes nothing.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N3-02 until this pull request is merged; then continue without a new authorization (D-034).

### N3-02 — Captive-portal mode (Q1)

- **Entry / prerequisites:** N3-01 complete within the approved N3 work order.
- **Approved output locations:** `crates/septuvo-policy/`, `crates/septuvo-daemon/`, `crates/septuvo-ipc/`, `crates/septuvo-cli/`, `packaging/linux/`, `tests/lab/namespaces/`, `tests/lab/install/`, `docs/build/captive-portal.md`, `docs/build/daemon.md`, `docs/build/install.md` (revised after the owner's review of PR #63).
- **Exact work:**
  - User-started windows of at most 5 minutes for **one dedicated helper account only** (kernel `skuid` match). Ordinary programs keep the kill switch unchanged.
  - The helper may use TCP 80/443 outside the tunnel and nothing else: never the tunnel, never policy-listed hosts.
  - Trusted DNS (DoH) by default. The network's DNS only with an explicit `--network-dns`, for the helper only; the resolver file is never changed.
  - Kernel-enforced expiry through nftables timeout sets; no adding up; at most 3 windows per connection, each logged; ends on disconnect; never automatic.
  - RFC 8910/8908 and a Septuvo DoH client are evaluated and planned for phase 2 (see the design note).
- **Acceptance evidence:**
  - Renderer tests: rules for ordinary programs are unchanged.
  - Namespace lab: ordinary programs cannot use the window; helper scope; explicit network DNS; malicious DNS answers redirect nothing; the resolver file is unchanged; re-blocking after a killed service; all refusals; status countdown.
  - Install test: the helper account is created and removed.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N3-03 until this pull request is merged; then continue without a new authorization (D-034).

### N3-03 — Fuzz testing (standard library only)

- **Entry / prerequisites:** N3-02 complete within the approved N3 work order.
- **Approved output locations:** fuzz tests in the crates named in the work order, `.github/workflows/fuzz.yml`, `docs/build/fuzzing.md`.
- **Exact work:** Seeded random, mutation and structure-aware fuzz targets for every parser; no-panic, bounded-output, round-trip and no-state-change properties; short runs per pull request, long scheduled runs, replayable seeds.
- **Acceptance evidence:** Every parser covered; short run green; a recorded long run of at least 30 minutes per target; any bug fixed with a regression test.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N3-04 until this pull request is merged; then continue without a new authorization (D-034).

### N3-04 — Live status view

- **Entry / prerequisites:** N3-03 complete within the approved N3 work order.
- **Approved output locations:** `crates/septuvo-cli/`, `tests/lab/namespaces/`, `docs/build/daemon.md`.
- **Exact work:** `septuvo watch`: refreshed four-stage status, service state, kill switch, DNS, resolver file and captive-portal countdown; read-only; plain fallback; lost service shown at once; never "connected" unless all four checks pass.
- **Acceptance evidence:** Rendering tests for every state including service lost; namespace lab case following live changes.
- **Stop point:** Submit the scoped evidence and PR for owner review. N3 is then complete; nothing beyond it starts without a new work order.


## Track N4 — Septuvo VPN desktop application for Linux (parallel, after G0, before G1)

Work allowed by D-029, defined in [work-orders/N4.md](work-orders/N4.md). Same boundary as N2 and N3: **never** an SVP handshake, packet encryption or wire encoding, key generation or management, usable invitations, functional protocol transport or any cryptography crate in Septuvo's code; the service stays lab-only. Third-party crates only in the separate `desktop/` workspace and only as approved in N4-01's review. One owner authorization covers the track (D-031); after each merge the agent continues to the next card (D-034).

### N4-01 — Toolkit review, application skeleton and CI

- **Entry / prerequisites:** N3-04 complete (N3 accepted) and the N4 work order merged.
- **Approved output locations:** `docs/reviews/dependency-review-desktop.md`, `desktop/`, `LICENSING.md`, `.github/workflows/desktop.yml`, `docs/build/desktop.md`.
- **Exact work:** Dependency review (versions at least two weeks old, licences, system libraries); separate `desktop/` workspace (PolyForm) with a libadwaita window, brand icon and theme, About with the development-preview notice; plain-Rust view model; refuses root.
- **Acceptance evidence:** Review recorded; CI builds with `--locked`, fmt/clippy, view-model tests, headless start-up under Xvfb; main `Cargo.lock` still free of third-party crates.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N4-02 until this pull request is merged; then continue without a new authorization (D-034).

### N4-02 — Status and monitoring

- **Entry / prerequisites:** N4-01 complete within the approved N4 work order.
- **Approved output locations:** `desktop/`, `crates/septuvo-cli/` (shared helpers), `docs/build/desktop.md`, `tests/lab/namespaces/`.
- **Exact work:** Status page over the status socket: four checks with help, service state, kill switch, DNS, resolver file, captive countdown, service-lost banner; "Connected" only when all four pass; adaptive 360 px-to-wide layout; text, not colour alone; screen-reader names.
- **Acceptance evidence:** View-model tests for every state; headless widget tests; namespace lab case with the app under Xvfb following each change.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N4-03 until this pull request is merged; then continue without a new authorization (D-034).

### N4-03 — Tray integration and notifications

- **Entry / prerequisites:** N4-02 complete within the approved N4 work order.
- **Approved output locations:** `desktop/`, `docs/build/desktop.md`.
- **Exact work:** StatusNotifierItem tray (`ksni`): icon and tooltip by state (no protected/connected icon before G1), menu, GIO notifications; closing the window keeps the tray (D-019); falls back to a window without a tray host.
- **Acceptance evidence:** State-to-icon/menu unit tests; CI test on a private D-Bus session with a test tray host.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N4-04 until this pull request is merged; then continue without a new authorization (D-034).

### N4-04 — Connection controls and permissions

- **Entry / prerequisites:** N4-03 complete within the approved N4 work order.
- **Approved output locations:** `desktop/`, `docs/build/desktop.md`, `tests/lab/namespaces/`.
- **Exact work:** Connect, disconnect (with confirmation) and captive-portal window (explicit network-DNS option) through the control socket; disabled controls with join-the-group guidance for non-members; no privilege prompts; plain-text errors.
- **Acceptance evidence:** View-model tests; lab case for a member and a non-member.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N4-05 until this pull request is merged; then continue without a new authorization (D-034).

### N4-05 — Configuration management

- **Entry / prerequisites:** N4-04 complete within the approved N4 work order.
- **Approved output locations:** `crates/septuvo-ipc/`, `crates/septuvo-daemon/`, `crates/septuvo-config/`, `desktop/`, `tests/lab/namespaces/`, `fuzz/`, `docs/build/`.
- **Exact work:** Service commands `CONFIG-GET` (status socket) and `CONFIG-APPLY` (control socket, length-prefixed, at most 64 KiB): validate, back up, write atomically, replace the kill switch atomically when connected; failure keeps the old file and rules. App settings page with inline validation, a diff and a plain summary before applying.
- **Acceptance evidence:** IPC tests and new fuzz targets (standard library and cargo-fuzz); engine tests for apply, reject, rollback; lab case: apply while connected with no leak window, a bad configuration changes nothing.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N4-06 until this pull request is merged; then continue without a new authorization (D-034).

### N4-06 — Diagnostics

- **Entry / prerequisites:** N4-05 complete within the approved N4 work order.
- **Approved output locations:** `crates/septuvo-ipc/`, `crates/septuvo-daemon/`, `crates/septuvo-cli/`, `desktop/`, `tests/lab/namespaces/`, `fuzz/`, `docs/build/`.
- **Exact work:** Read-only service `DIAGNOSE`; client checks (reachability, group membership, sockets, versions); findings with "what this means / what to do"; `septuvo diagnose` and an app page; a local text report the user saves; nothing sent anywhere.
- **Acceptance evidence:** Unit tests per finding; fuzzing of the new response; lab case with deliberate faults.
- **Stop point:** Submit the scoped evidence and PR for owner review. Do not begin N4-07 until this pull request is merged; then continue without a new authorization (D-034).

### N4-07 — Packaging, accessibility and the user guide

- **Entry / prerequisites:** N4-06 complete within the approved N4 work order.
- **Approved output locations:** `packaging/linux/`, `tests/lab/install/`, `desktop/`, `docs/build/desktop.md`, `docs/user/linux-desktop.md`.
- **Exact work:** Optional desktop install (`.desktop` entry, brand icons, tray autostart off by default) and complete uninstall; keyboard-only operation, screen-reader labels, contrast, text scaling; responsiveness (no blocked UI); user guide with CI screenshots.
- **Acceptance evidence:** Install test extended; accessibility checklist with evidence; guide reviewed by the owner.
- **Stop point:** Submit the scoped evidence and PR for owner review. After it is merged N4 is complete; nothing further starts without a new work order.
## Task catalog invariants

1. Each of the **43 base tasks** has a unique ID and a prerequisite, bounded output, actions, acceptance evidence and a stop point. Conditional dotted subtasks and owner-directed H work are separately recorded/approved in [STATUS.md](STATUS.md) and do not change the 43-base-task count; nor do track N1, N2 and N3 cards.
2. No agent can approve G0–G4, external review, a domain/trademark claim, production readiness or a public release on the owner's behalf.
3. M2 spec proposals, the ML-KEM/ML-DSA/HKDF/AES-GCM candidate choices (D-010, D-011, D-012, D-024) and packet wire fields must not become a deployed handshake before G1, even if a code generator could implement them quickly.
4. A Rust workspace, GUI, installer, mobile extension, control-plane cluster or gateway capability does **not** exist merely because this catalog names a target path.
5. At any interruption, read repository history, compare accepted tasks and resume only with explicit authorization for the next scoped work order.
6. Track N1 never contains cryptography, SVP wire handling or a data tunnel; anything beyond that requires G1.
