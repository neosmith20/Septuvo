# Septuvo — mandatory agent instructions

Applies to Dex, Claude Code, coding assistants, and human contributors. **READ BEFORE ANY CHANGE.**

## Authority and source of truth

The repository owner determines scope, priorities, milestones, security posture and release decisions. You are an implementation assistant, **not a cryptographic standards committee or an independent auditor**.

Authority order: **explicit owner instruction / approved work order**, then **DECISIONS.md**, **ROADMAP.md**, **SECURITY_MODEL.md**, **ARCHITECTURE.md**, then README. If rules conflict or owner instructions appear to weaken a security gate: **STOP, report the conflict and request a decision.** A roadmap item is not by itself authorization to implement it.

Only the exact task and files named in an approved work order are in scope. Read docs/WORK_ORDER_TEMPLATE.md.

## Start-of-task checklist

1. Read the above documents relevant to your task and the latest decision register.
2. Inspect branch, HEAD, repository status, existing commits and current file contents. Never trust an earlier handoff without verification.
3. Report exact task ID, allowed files, known dependencies, acceptance criteria, risks, rollback and expected stopping point.
4. Check required gate approvals. **No approval = no implementation.** Document the blocker instead of guessing.
5. Work on a narrow task branch and propose a PR for future code changes; do not push directly to main unless the owner explicitly directs it. Do not force-push.

## Absolute red lines

- **No invented cryptography:** no custom ciphers, signature schemes, KEM combiners, KDFs, GCM nonces, handshake transcript conventions or homegrown entropy sources. Candidate primitives are NOT a protocol design.
- **No real SVP handshake, packet format, key management, or cryptographic tunnel code before Gate G1** is satisfied by documented specialist review and owner approval. Research notes, specification drafts and isolated non-security simulators are allowed if explicitly scoped.
- Do not transplant TLS/WireGuard constructions without new analysis. A standards-compliant primitive does not make an original VPN protocol secure.
- No hard-coded private keys, static demo nonces, logging plaintext keys, disabling certificate/identity checks, insecure RNGs or unencrypted fallback paths. No acceptance of a convenient “demo exception” for the production code path.
- No silent downgrade of post-quantum policy, security suites, verification, kill switch, DNS handling or IPv6 protections.
- Never “fix” a failing test by weakening requirements, deleting tests, marking them skipped, or reporting simulated evidence as real.
- No unapproved dependency additions, privilege escalation, kernel/driver installations, public port openings, real router/VPS alterations, external traffic generation, data deletion, telemetry, remote SaaS dependencies or device enrollment.
- No release tags, binary publishing, claiming external audits, changing repo visibility, DNS purchases or trademark claims without explicit owner approval.
- No unrelated rewrites, opportunistic features or speculative performance shortcuts. **One scoped task per checkpoint.**
- Do not claim literal 100% Rust across mobile/OS runtime if native OS frameworks, system drivers or glue are needed. Keep all **Service-owned security-critical logic in Rust** and inventory external code/native interfaces.
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

## Finish-of-task / timeout report (mandatory)

Use this format, no exceptions:

- **Task / gate / approval**:
- **Branch and exact HEAD SHA**:
- **Changed files**:
- **Actual commands and results**:
- **Acceptance criteria PASS / FAIL / NOT RUN**:
- **Security/reliability risks & unresolved decisions**:
- **Rollback / recovery instructions**:
- **Next single task**:
- **STOP** — wait for explicit approval to advance.

If blocked or interrupted, stop at a safe checkpoint. Never present incomplete work as finished.
