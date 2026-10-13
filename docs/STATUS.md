# Septuvo — owner-controlled task and gate ledger

**This is the authoritative record of owner approvals.** Initial baseline only: all 43 base tasks are NOT STARTED and all gates are PENDING. No approval is implied by creating this file.

**OWNER ONLY:** Agents must not edit this file, even on task branches. The owner updates it using an owner-authored commit or pull request under normal GitHub protections, with dates and permalinks. Only the owner may record work-order authorization, acceptance, task waiver, or G0–G4 passage. Missing/stale evidence means STOP.

## Work-order and task approval procedure

1. Agent drafts [WORK_ORDER_TEMPLATE.md](WORK_ORDER_TEMPLATE.md) for one exact task/scope and stops.
2. Owner comments `APPROVED WORK ORDER <ID>` with a link to the exact work-order revision in a GitHub issue/PR, then **personally records** status **READY**, UTC date and the comment/work-order links below. A PR, merge or CI pass alone is not approval.
3. Agent verifies ledger, approval, predecessor, task scope and gate before proceeding. Changes to scope require a fresh explicit approval, recorded by owner.
4. Agent submits completion evidence/PR and stops. Owner alone marks **ACCEPTED** (or **REJECTED / REWORK**) and records date plus evidence.
5. Owner alone sets a gate to **PASSED** (or **FAILED**) after reviewing exact gate evidence and required independent reviewer report. Owner must record date and links. G1 and G3 still require independent review; no agent may self-approve.
6. If an owner uses GitHub's protected PR process to update this ledger, the **owner** authors the ledger edit and merges it; agents must not modify or bypass it.

Use dates in `YYYY-MM-DD` UTC and permalinks. A blank approval reference means **not authorized**.

## Security gates

| Gate | State | Owner decision date (UTC) | Evidence and reviewed revision |
| --- | --- | --- | --- |
| G0 | PENDING | — | — |
| G1 | PENDING | — | — |
| G2 | PENDING | — | — |
| G3 | PENDING | — | — |
| G4 | PENDING | — | — |

## Fixed base-task status (43)

| Task ID | State | Predecessor | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- | --- |
| M0-01 | NOT STARTED | — | — | — | — |
| M0-02 | NOT STARTED | M0-01 | — | — | — |
| M0-03 | NOT STARTED | M0-02 | — | — | — |
| M1-01 | NOT STARTED | M0-03 | — | — | — |
| M1-02 | NOT STARTED | M1-01 | — | — | — |
| M1-03 | NOT STARTED | M1-02 | — | — | — |
| M1-04 | NOT STARTED | M1-03 | — | — | — |
| M2-01 | NOT STARTED | M1-04 | — | — | — |
| M2-02 | NOT STARTED | M2-01 | — | — | — |
| M2-03 | NOT STARTED | M2-02 | — | — | — |
| M2-04 | NOT STARTED | M2-03 | — | — | — |
| M2-05 | NOT STARTED | M2-04 | — | — | — |
| M3-01 | NOT STARTED | M2-05 | — | — | — |
| M3-02 | NOT STARTED | M3-01 | — | — | — |
| M3-03 | NOT STARTED | M3-02 | — | — | — |
| M3-04 | NOT STARTED | M3-03 | — | — | — |
| M4-01 | NOT STARTED | M3-04 | — | — | — |
| M4-02 | NOT STARTED | M4-01 | — | — | — |
| M4-03 | NOT STARTED | M4-02 | — | — | — |
| M4-04 | NOT STARTED | M4-03 | — | — | — |
| M5-01 | NOT STARTED | M4-04 | — | — | — |
| M5-02 | NOT STARTED | M5-01 | — | — | — |
| M5-03 | NOT STARTED | M5-02 | — | — | — |
| M5-04 | NOT STARTED | M5-03 | — | — | — |
| M5-05 | NOT STARTED | M5-04 | — | — | — |
| M6-01 | NOT STARTED | M5-05 | — | — | — |
| M6-02 | NOT STARTED | M6-01 | — | — | — |
| M6-03 | NOT STARTED | M6-02 | — | — | — |
| M6-04 | NOT STARTED | M6-03 | — | — | — |
| M7-01 | NOT STARTED | M6-04 | — | — | — |
| M7-02 | NOT STARTED | M7-01 | — | — | — |
| M7-03 | NOT STARTED | M7-02 | — | — | — |
| M7-04 | NOT STARTED | M7-03 | — | — | — |
| M7-05 | NOT STARTED | M7-04 | — | — | — |
| M8-01 | NOT STARTED | M7-05 | — | — | — |
| M8-02 | NOT STARTED | M8-01 | — | — | — |
| M8-03 | NOT STARTED | M8-02 | — | — | — |
| M8-04 | NOT STARTED | M8-03 | — | — | — |
| M9-01 | NOT STARTED | M8-04 | — | — | — |
| M9-02 | NOT STARTED | M9-01 | — | — | — |
| M9-03 | NOT STARTED | M9-02 | — | — | — |
| M9-04 | NOT STARTED | M9-03 | — | — | — |
| M9-05 | NOT STARTED | M9-04 | — | — | — |

## Dotted subtasks (owner authorized separately)

M0-01.1 is a conditional correction task: if M0-01 finds defects, the owner authorizes and accepts M0-01.1 before M0-02. If none, the owner explicitly records the skip in M0-01 acceptance evidence. For M7-01, M8-02, M9-02 and similar cases, every `M7-01.1` style subtask must be owner-created here and accepted before its parent can become ACCEPTED. Agents cannot create ledger entries.

| Subtask ID | Parent | State | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- | --- |
| M0-01.1 | M0-01 | NOT STARTED | — | — | Conditional / not authorized |

## Owner-directed hygiene, branding and administration

Use separate `H-001`, `H-002` etc. IDs for work outside the 43 base tasks; they never change the base task order or approve a gate. Owner must record a separately approved scope and completion evidence. The current documentation audit remediation is proposed as H-001; the owner has not yet entered acceptance.

| ID | State | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- |
| H-001 | NOT STARTED | — | — | Proposed documentation workflow fixes |

## Allowed task states

`NOT STARTED`, `READY`, `IN PROGRESS`, `BLOCKED`, `IN REVIEW`, `ACCEPTED`, `REJECTED / REWORK`.

IN REVIEW waits for owner decision; BLOCKED waits for its stated prerequisite; REJECTED / REWORK resumes only after revised approval. Do **not** skip such tasks.

See [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md), [TASK_CATALOG.md](TASK_CATALOG.md), [WORK_ORDER_TEMPLATE.md](WORK_ORDER_TEMPLATE.md).
