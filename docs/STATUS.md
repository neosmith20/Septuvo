# Septuvo — task and gate ledger

**This file records task and gate status from GitHub evidence (D-031).** Agents keep it up to date; the owner never edits these tables by hand. It was created as a baseline with every task NOT STARTED and every gate PENDING; the tables below show the current state.

## How work is approved and accepted (D-031)

| Category | Covers | Authorization | Acceptance |
| --- | --- | --- | --- |
| **A — Routine** (`H-xxx`) | Documentation, bot findings (Aikido, Dependabot), CI maintenance, bounded fixes | None needed: an agent prepares a scoped pull request | **The owner's merge** of the pull request |
| **B — Milestone work** (`Mx-yy`, `Nx-yy`, dotted subtasks) | Tasks inside an authorized milestone | One owner authorization per milestone or scope change, **in ordinary words**: an owner comment naming the milestone, a short comment on the pull request that carries its work order, or the owner's merge of a pull request that adds the work order under `docs/work-orders/` | **The owner's merge** of the pull request that delivers the task |
| **C — Gates and security decisions** | G0–G4; cryptographic constructions, trust model, security architecture, releases | Evidence first (G1: qualified independent protocol review; G3: independent implementation assessment) | Gates: the owner's exact comment `GATE <Gn> PASSED` or `GATE <Gn> FAILED`. Security decisions: an explicit owner comment, recorded in [DECISIONS.md](DECISIONS.md) |

Rules that hold for every category:

- No agent or bot merges a pull request, passes a gate or makes a category C decision. The owner merges every pull request.
- A pull request **delivers** a task when the task ID is the scope of one of its commit subjects, for example `feat(N1-03): ...`. Mentions elsewhere, such as "next: N1-03", never count.
- Milestones cannot advance past an unpassed gate: M1, M2 and N1 need G0; M3–M5 need G1; M6–M8 need G2; M9 needs G3. No cryptographic or protocol implementation before G1.
- **Ledger upkeep is automatic.** [`scripts/sync_status.py`](../scripts/sync_status.py) reads owner-merged pull requests and updates this file. Agents run it and include the result in a pull request, and CI reports any drift on every pull request. A ledger that lags behind GitHub never blocks already-authorized work: merged pull requests are the authority.
- **CI verifies every record** ([`scripts/verify_ledger.py`](../scripts/verify_ledger.py)): the owner merged the linked pull request into `main` and it delivers the task; an authorization was written or merged by the owner; a gate has the exact owner comment. Records that cannot be verified fail the check, and so does any network error.
- Earlier records based on exact comments (`APPROVED WORK ORDER`, `ACCEPTED`, D-025) stay valid. Rows H-001–H-004, M0-01 and M0-01.1 predate D-025 and are kept verbatim.

Dates are `YYYY-MM-DD` UTC: the authorization date for READY rows, the merge date for ACCEPTED rows.

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
| M1-01 | ACCEPTED | M0-03 | 2026-10-14 | [Milestone M1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162806536) | [PR #30](https://github.com/neosmith20/Septuvo/pull/30) |
| M1-02 | ACCEPTED | M1-01 | 2026-10-14 | [Milestone M1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162806536) | [PR #30](https://github.com/neosmith20/Septuvo/pull/30) |
| M1-03 | ACCEPTED | M1-02 | 2026-10-14 | [Milestone M1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162806536) | [PR #30](https://github.com/neosmith20/Septuvo/pull/30) |
| M1-04 | ACCEPTED | M1-03 | 2026-10-14 | [Milestone M1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162806536) | [PR #30](https://github.com/neosmith20/Septuvo/pull/30) |
| M2-01 | READY | M1-04 | 2026-10-15 | [Work order M2, merged in PR #46](https://github.com/neosmith20/Septuvo/pull/46) | — |
| M2-02 | READY | M2-01 | 2026-10-15 | [Work order M2, merged in PR #46](https://github.com/neosmith20/Septuvo/pull/46) | — |
| M2-03 | READY | M2-02 | 2026-10-15 | [Work order M2, merged in PR #46](https://github.com/neosmith20/Septuvo/pull/46) | — |
| M2-04 | READY | M2-03 | 2026-10-15 | [Work order M2, merged in PR #46](https://github.com/neosmith20/Septuvo/pull/46) | — |
| M2-05 | READY | M2-04 | 2026-10-15 | [Work order M2, merged in PR #46](https://github.com/neosmith20/Septuvo/pull/46) | — |
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
| N1-01 | ACCEPTED | — | 2026-10-14 | [Milestone N1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162807229) | [PR #29](https://github.com/neosmith20/Septuvo/pull/29) |
| N1-02 | ACCEPTED | N1-01 | 2026-10-14 | [Milestone N1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162807229) | [PR #29](https://github.com/neosmith20/Septuvo/pull/29) |
| N1-03 | ACCEPTED | N1-02 | 2026-10-15 | [Milestone N1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162807229) | [PR #45](https://github.com/neosmith20/Septuvo/pull/45) |
| N1-04 | ACCEPTED | N1-03 | 2026-10-15 | [Milestone N1 approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162807229) | [PR #45](https://github.com/neosmith20/Septuvo/pull/45) |

## Dotted subtasks (owner authorized separately)

M0-01.1 is a conditional correction task: if M0-01 finds defects, the owner authorizes and accepts M0-01.1 before M0-02. If none, the owner explicitly records the skip in M0-01 acceptance evidence. For M7-01, M8-02, M9-02 and similar cases, every `M7-01.1` style subtask needs its own owner authorization (category B) and must be accepted before its parent can become ACCEPTED.

| Subtask ID | Parent | State | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- | --- |
| M0-01.1 | M0-01 | ACCEPTED | 2026-10-14 | [Approval on issue #13](https://github.com/neosmith20/Septuvo/issues/13#issuecomment-6161852650) | [PR #15](https://github.com/neosmith20/Septuvo/pull/15) |

## Owner-directed hygiene, branding and administration

Use separate `H-001`, `H-002` etc. IDs for routine work outside the 43 base tasks (category A). They never change the base task order, add milestone scope or approve a gate. The owner's merge accepts them, and `sync_status.py` adds their rows. H-001 and H-003 were accepted retroactively on 2026-10-14 (no written work order existed beforehand).

| ID | State | Owner decision date (UTC) | Work-order approval | Completion evidence |
| --- | --- | --- | --- | --- |
| H-001 | ACCEPTED | 2026-10-14 | Retroactive owner acceptance 2026-10-14 (no prior written work order) | Documentation workflow fixes: [PR #4](https://github.com/neosmith20/Septuvo/pull/4) |
| H-002 | ACCEPTED | 2026-10-14 | Owner instruction 2026-10-14: record D-021 option C; fix catalog predecessors and work-order status line; align color-board type sizes | [PR #5](https://github.com/neosmith20/Septuvo/pull/5) |
| H-003 | ACCEPTED | 2026-10-14 | Retroactive owner acceptance 2026-10-14 (owner reviewed and committed the files) | Brand kit corrections: guidelines PDF, brand README, root .gitignore ([c3195eb](https://github.com/neosmith20/Septuvo/commit/c3195eb)); color board colors ([160fbde](https://github.com/neosmith20/Septuvo/commit/160fbde)) |
| H-004 | ACCEPTED | 2026-10-14 | Owner instruction 2026-10-14: document public repository, active ruleset and security features; record D-022/D-023 | [PR #7](https://github.com/neosmith20/Septuvo/pull/7) |
| H-005 | ACCEPTED | 2026-10-14 | [Approval on issue #17](https://github.com/neosmith20/Septuvo/issues/17#issuecomment-6162227517) | [PR #20](https://github.com/neosmith20/Septuvo/pull/20) and scope update [PR #22](https://github.com/neosmith20/Septuvo/pull/22); [acceptance on PR #20](https://github.com/neosmith20/Septuvo/pull/20#issuecomment-6162868345) |
| H-006 | ACCEPTED | 2026-10-14 | [Approval on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162805074) | Lean execution process (D-030): [PR #24](https://github.com/neosmith20/Septuvo/pull/24); [acceptance on PR #24](https://github.com/neosmith20/Septuvo/pull/24#issuecomment-6162833818) |
| H-007 | ACCEPTED | 2026-10-14 | Routine (D-031) | [PR #28](https://github.com/neosmith20/Septuvo/pull/28) |
| H-008 | ACCEPTED | 2026-10-14 | Routine (D-031) | [PR #36](https://github.com/neosmith20/Septuvo/pull/36) |
| H-009 | ACCEPTED | 2026-10-15 | Routine (D-031) | [PR #41](https://github.com/neosmith20/Septuvo/pull/41) |
| H-010 | ACCEPTED | 2026-10-15 | Routine (D-031) | [PR #42](https://github.com/neosmith20/Septuvo/pull/42) |
| H-011 | ACCEPTED | 2026-10-15 | Routine (D-031) | [PR #43](https://github.com/neosmith20/Septuvo/pull/43) |
| H-012 | ACCEPTED | 2026-10-15 | Routine (D-031) | [PR #44](https://github.com/neosmith20/Septuvo/pull/44) |
| H-013 | ACCEPTED | 2026-10-15 | Routine (D-031) | [PR #46](https://github.com/neosmith20/Septuvo/pull/46) |

## Allowed task states

`NOT STARTED`, `READY`, `IN PROGRESS`, `BLOCKED`, `IN REVIEW`, `ACCEPTED`, `REJECTED / REWORK`.

IN REVIEW waits for owner decision; BLOCKED waits for its stated prerequisite; REJECTED / REWORK resumes only after revised approval. Do **not** skip such tasks.

See [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md), [TASK_CATALOG.md](TASK_CATALOG.md), [WORK_ORDER_TEMPLATE.md](WORK_ORDER_TEMPLATE.md).
