# Septuvo — mandatory agent instructions

Applies to **Codex, Claude Code, Dex, and any other coding agent or human contributor**, regardless of vendor, IDE or execution tool. **READ BEFORE ANY CHANGE.**

## Authority and source of truth

The repository owner determines scope, priorities, milestones, security posture and release decisions. You are an implementation assistant, **not a cryptographic standards committee or an independent auditor**.

Authority: **the owner's merges, authorizations and gate comments on GitHub, recorded in [STATUS.md](docs/STATUS.md) (D-031)** for authorization/gates; **AGENTS.md and [AGENT_OPERATIONS.md](docs/AGENT_OPERATIONS.md)** for mandatory execution rules; **[DECISIONS.md](docs/DECISIONS.md)** for recorded technical choices; **[ROADMAP.md](docs/ROADMAP.md) and [TASK_CATALOG.md](docs/TASK_CATALOG.md)** for task sequence/scope; **[SECURITY_MODEL.md](docs/SECURITY_MODEL.md) and [ARCHITECTURE.md](docs/ARCHITECTURE.md)** for design; **[TRACEABILITY.md](docs/TRACEABILITY.md)** for evidence; **[WORK_ORDER_TEMPLATE.md](docs/WORK_ORDER_TEMPLATE.md)** as the single handoff format; README for overview. If any documents conflict or a request could weaken a gate: **STOP and request owner resolution**.

Only the exact task and files named in an authorized work order (or, for routine H work, the pull request's stated scope) are in scope. Read [AGENT_OPERATIONS.md](docs/AGENT_OPERATIONS.md), [TASK_CATALOG.md](docs/TASK_CATALOG.md), [TRACEABILITY.md](docs/TRACEABILITY.md), and [WORK_ORDER_TEMPLATE.md](docs/WORK_ORDER_TEMPLATE.md). Roadmap bullets are not blanket work authorization. Task cards propose output paths; they do not authorize edits or gate passage.

## Start-of-task checklist

1. Read the above documents relevant to your task and the latest decision register.
2. Inspect branch, HEAD, repository status, existing commits and current file contents. Never trust an earlier handoff without verification.
3. Report exact task ID, allowed files, known dependencies, acceptance criteria, risks, rollback and expected stopping point.
4. Check the required gate and, for milestone work, the owner's authorization of that milestone (D-031). **No authorization = no implementation** of milestone work. Routine work (H-xxx: docs, bot findings, CI upkeep, bounded fixes) needs no authorization comment; the owner's merge accepts it. Document the blocker instead of guessing.
5. Work on a narrow task branch and **open the PR yourself** (GitHub API, CLI or connected GitHub tools) for **every change**, code or documentation, with the complete description and evidence, then give the owner the PR URL. PR text never contains AI session links or IDs. Agents never merge a pull request and never push to, merge into, or force-push `main` (the `main` ruleset also blocks direct pushes) (D-034).

## Absolute red lines

- **No invented cryptography:** no custom ciphers, signature schemes, KEM combiners, KDFs, GCM nonces, handshake transcript conventions or homegrown entropy sources. Candidate primitives are NOT a protocol design.
- **No real SVP handshake, executable packet format, key management, or cryptographic tunnel code before Gate G1** is satisfied by documented specialist review and owner approval. Research notes, specification drafts and isolated non-security simulators are allowed if explicitly scoped, as is track N1 non-crypto lab work after G0 (D-029, D-030).
- Do not transplant TLS/WireGuard constructions without new analysis. A standards-compliant primitive does not make an original VPN protocol secure.
- No hard-coded private keys, static demo nonces, logging plaintext keys, disabling certificate/identity checks, insecure RNGs or unencrypted fallback paths. No acceptance of a convenient “demo exception” for the production code path.
- No silent downgrade of post-quantum policy, security suites, verification, kill switch, DNS handling or IPv6 protections.
- Never “fix” a failing test by weakening requirements, deleting tests, marking them skipped, or reporting simulated evidence as real.
- No unapproved dependency additions, privilege escalation, kernel/driver installations, public port openings, real router/VPS alterations, external traffic generation, data deletion, telemetry, remote SaaS dependencies or device enrollment.
- No release tags, binary publishing, claiming external audits, changing repo visibility, DNS purchases or trademark claims without explicit owner approval.
- No unrelated rewrites, opportunistic features or speculative performance shortcuts. **One authorized work order (a task or a milestone) or one routine fix per pull request, finished one task at a time.**
- Do not claim literal 100% Rust across mobile/OS runtime if native OS frameworks, system drivers or glue are needed. Keep all **Septuvo-owned security-critical logic in Rust** and inventory external code/native interfaces.
- Treat remote packets, configuration, peer identity, management APIs and control-plane data as hostile input.

## Engineering practices

- First-party cryptographic orchestration, session state, packet processing, policy and service code must be Rust. Prefer audited and maintained dependencies, pin versions, record licenses, and require approval for security-sensitive dependencies.
- Separate privileged daemon and unprivileged UI. Prefer OS-native VPN APIs. No unnecessary custom drivers.
- An interface being “up” is **not** proof of peer authentication, encrypted reachability, DNS or internet access. Display and test those independently.
- Default network posture is fail closed; do not equate private RFC1918 addresses with trusted LANs.
- Add negative and fault-injection tests for new inputs/state transitions and prove that failure does not leak protected traffic.
- Verify actual tests and build output. Report exact commands, version, platform and results, including failures. Never state “tested” based on code inspection alone.
- Preserve previous working behavior, minimize diffs and provide clear review notes.

## Gate policy

| Gate | Owner must receive evidence before approval |
| --- | --- |
| **G0 — Design baseline** | Product scope, decisions, unresolved questions, acceptance criteria reviewed |
| **G1 — Protocol design review** | Threat model, authenticated hybrid handshake, packet format, key/nonce lifecycle, downgrade/DoS analysis and qualified external specialist review |
| **G2 — Controlled lab qualification** | Real two-node tunnel, negative cases, leak/kill-switch tests, reproducible results, explicit lab-only limitations |
| **G3 — Production release** | Independent implementation/security assessment, remediation, platform matrix, signed builds, incident/update processes and owner approval |
| **G4 — Enterprise/provider claims** | Capacity, multi-tenant isolation, HA and abuse resistance tested; owners approve evidenced limits |

## Task dispatch and state

Follow the deterministic task-dispatch algorithm in [AGENT_OPERATIONS.md](docs/AGENT_OPERATIONS.md): inspect actual state, select only the first eligible task, verify owner authorization **and** required gate, execute only that task, open its PR with evidence, and **STOP for the owner's review**. When the owner merges it, verify the predecessor is complete, sync the ledger and **continue with the next task of the same authorized milestone without asking again** (D-034). Still stop for a genuine owner security-architecture or cryptography decision, a material scope change, a gate or a new milestone; never infer any of these from a merge. Every agent uses the same statuses (NOT STARTED, READY, IN PROGRESS, BLOCKED, IN REVIEW, ACCEPTED, REJECTED / REWORK). Only the owner authorizes milestones, accepts work (by merging its pull request) or passes gates (exact `GATE <Gn> PASSED/FAILED` comment), as defined in [STATUS.md](docs/STATUS.md) (D-031). No agent or bot merges a pull request. Agents keep the ledger in sync with `scripts/sync_status.py` and never record anything GitHub evidence does not support. On timeout, inspect actual HEAD/worktree and reconcile before resuming; do not assume a command completed.

## Finish-of-task / timeout report (mandatory)

Use the complete **Completion and handoff** section of [WORK_ORDER_TEMPLATE.md](docs/WORK_ORDER_TEMPLATE.md) as the **only** required format. It covers task/gate approval, full SHA, PR URL, real test evidence, risks, rollback actually tested, and one proposed next task. **STOP** for owner review.

If blocked or interrupted, stop at a safe checkpoint. Never present incomplete work as finished.
