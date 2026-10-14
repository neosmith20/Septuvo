# Septuvo — task and gate ledger (mirror of owner decisions)

**This file mirrors the owner's decision comments.** It was created as a baseline with every task NOT STARTED and every gate PENDING; the tables below show the current state. No approval is implied by creating this file.

**Owner decisions are comments (D-025).** The owner decides only by posting one of these exact comments, alone in the comment, in the conversation of the task's GitHub issue or pull request: `APPROVED WORK ORDER <ID>`, `ACCEPTED <ID>`, `REJECTED <ID>`, `GATE <G0–G4> PASSED` or `GATE <G0–G4> FAILED`. Those comments are the authority; this file mirrors them. Agents may change this file **only** to mirror such comments, in their next pull request, with each comment's permalink and UTC date; any other change by an agent is forbidden, and the owner still reviews and merges every change. CI ([`scripts/verify_ledger.py`](../scripts/verify_ledger.py)) fails a pull request if a mirrored row has no matching owner comment. Rows H-001–H-004, M0-01 and M0-01.1 predate this rule and were edited by the owner directly. Missing or stale evidence means STOP.

## Work-order and task approval procedure

1. Agent drafts [WORK_ORDER_TEMPLATE.md](WORK_ORDER_TEMPLATE.md) for one exact task/scope and stops.
2. Owner comments `APPROVED WORK ORDER <ID>` on the GitHub issue or PR that contains the work order. The approved revision is the issue/PR description as it stood when the approval comment was posted. A PR, merge or CI pass alone is not approval. A work order may cover a whole milestone (`APPROVED WORK ORDER M1`), or be a file under `docs/work-orders/` in a pull request, in which case the approved revision is that file at the pull request's head commit when the comment was posted (D-030).
3. Agent verifies the owner's comment on GitHub (author is the owner, exact text), the predecessor, task scope and gate before proceeding; its pull request mirrors the approval below (READY, UTC date, permalink). Changes to scope require a fresh explicit owner approval comment.
4. Agent submits completion evidence/PR and stops. After review the owner comments `ACCEPTED <ID>` (or `REJECTED <ID>`) on the pull request, where `<ID>` may be a milestone such as `M1`; the next agent pull request mirrors it with the evidence. A merge alone is not acceptance.
5. Owner alone passes or fails a gate, by commenting `GATE <Gn> PASSED` (or `FAILED`) on the gate-evidence pull request after reviewing the exact evidence and any required independent reviewer report. G1 and G3 still require independent review; no agent may self-approve.
6. The owner may also edit this file directly through a pull request; agents must never record a decision that has no matching owner comment.

Use dates in `YYYY-MM-DD` UTC and permalinks. A blank approval reference means **not authorized**.

## Security gates

| Gate | State | Owner decision date (UTC) | Evidence and reviewed revision |
| --- | --- | --- | --- |
| G0 | PASSED | 2026-10-14 | [Gate decision on PR #25](https://github.com/neosmith20/Septuvo/pull/25#issuecomment-6162915062) — evidence [`docs/reviews/G0.md`](reviews/G0.md) as merged in [PR #25](https://github.com/neosmith20/Septuvo/pull/25) |
| G1 | PENDING | — | — |
| G2 | PENDING | — | — |
| G3 | PENDING | — | — |
| G4 | PENDING | — | — |

## Fixed base-task status (43)

| Task ID | State | Predecessor | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- | --- |
| M0-01 | ACCEPTED | — | 2026-10-14 | [Approval on issue #9](https://github.com/neosmith20/Septuvo/issues/9#issuecomment-6161523935) | [PR #12](https://github.com/neosmith20/Septuvo/pull/12) — `docs/reviews/M0-01-audit.md` |
| M0-02 | ACCEPTED | M0-01 | 2026-10-14 | [Approval on issue #18](https://github.com/neosmith20/Septuvo/issues/18#issuecomment-6162237599) | [PR #21](https://github.com/neosmith20/Septuvo/pull/21) — `docs/reviews/M0-02-owner-decisions.md`; [acceptance on PR #21](https://github.com/neosmith20/Septuvo/pull/21#issuecomment-6162871333) |
| M0-03 | ACCEPTED | M0-02 | 2026-10-14 | [Approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162805811) | [PR #25](https://github.com/neosmith20/Septuvo/pull/25) — `docs/reviews/G0.md`; [acceptance on PR #25](https://github.com/neosmith20/Septuvo/pull/25#issuecomment-6162913527) |
| M1-01 | READY | M0-03 | 2026-10-14 | [Milestone M1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162806536) | — |
| M1-02 | READY | M1-01 | 2026-10-14 | [Milestone M1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162806536) | — |
| M1-03 | READY | M1-02 | 2026-10-14 | [Milestone M1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162806536) | — |
| M1-04 | READY | M1-03 | 2026-10-14 | [Milestone M1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162806536) | — |
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

## Track N1 (non-crypto, after G0, before G1)

| Task ID | State | Predecessor | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- | --- |
| N1-01 | READY | — | 2026-10-14 | [Milestone N1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162807229) | — |
| N1-02 | READY | N1-01 | 2026-10-14 | [Milestone N1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162807229) | — |
| N1-03 | READY | N1-02 | 2026-10-14 | [Milestone N1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162807229) | — |
| N1-04 | READY | N1-03 | 2026-10-14 | [Milestone N1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162807229) | — |

## Dotted subtasks (owner authorized separately)

M0-01.1 is a conditional correction task: if M0-01 finds defects, the owner authorizes and accepts M0-01.1 before M0-02. If none, the owner explicitly records the skip in M0-01 acceptance evidence. For M7-01, M8-02, M9-02 and similar cases, every `M7-01.1` style subtask needs its own owner approval comment and must be accepted before its parent can become ACCEPTED. Agents add rows only to mirror owner comments.

| Subtask ID | Parent | State | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- | --- |
| M0-01.1 | M0-01 | ACCEPTED | 2026-10-14 | [Approval on issue #13](https://github.com/neosmith20/Septuvo/issues/13#issuecomment-6161852650) | [PR #15](https://github.com/neosmith20/Septuvo/pull/15) |

## Owner-directed hygiene, branding and administration

Use separate `H-001`, `H-002` etc. IDs for work outside the 43 base tasks; they never change the base task order or approve a gate. Each H task needs its own owner approval comment; agents mirror it here. H-001 and H-003 were accepted retroactively on 2026-10-14 (no written work order existed beforehand).

| ID | State | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- |
| H-001 | ACCEPTED | 2026-10-14 | Retroactive owner acceptance 2026-10-14 (no prior written work order) | Documentation workflow fixes: [PR #4](https://github.com/neosmith20/Septuvo/pull/4) |
| H-002 | ACCEPTED | 2026-10-14 | Owner instruction 2026-10-14: record D-021 option C; fix catalog predecessors and work-order status line; align color-board type sizes | [PR #5](https://github.com/neosmith20/Septuvo/pull/5) |
| H-003 | ACCEPTED | 2026-10-14 | Retroactive owner acceptance 2026-10-14 (owner reviewed and committed the files) | Brand kit corrections: guidelines PDF, brand README, root .gitignore ([c3195eb](https://github.com/neosmith20/Septuvo/commit/c3195eb)); color board colors ([160fbde](https://github.com/neosmith20/Septuvo/commit/160fbde)) |
| H-004 | ACCEPTED | 2026-10-14 | Owner instruction 2026-10-14: document public repository, active ruleset and security features; record D-022/D-023 | [PR #7](https://github.com/neosmith20/Septuvo/pull/7) |
| H-005 | ACCEPTED | 2026-10-14 | [Approval on issue #17](https://github.com/neosmith20/Septuvo/issues/17#issuecomment-6162227517) | [PR #20](https://github.com/neosmith20/Septuvo/pull/20) and scope update [PR #22](https://github.com/neosmith20/Septuvo/pull/22); [acceptance on PR #20](https://github.com/neosmith20/Septuvo/pull/20#issuecomment-6162868345) |
| H-006 | ACCEPTED | 2026-10-14 | [Approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162805074) | Lean execution process (D-030): [PR #24](https://github.com/neosmith20/Septuvo/pull/24); [acceptance on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162833818) |

## Allowed task states

`NOT STARTED`, `READY`, `IN PROGRESS`, `BLOCKED`, `IN REVIEW`, `ACCEPTED`, `REJECTED / REWORK`.

IN REVIEW waits for owner decision; BLOCKED waits for its stated prerequisite; REJECTED / REWORK resumes only after revised approval. Do **not** skip such tasks.

See [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md), [TASK_CATALOG.md](TASK_CATALOG.md), [WORK_ORDER_TEMPLATE.md](WORK_ORDER_TEMPLATE.md).
