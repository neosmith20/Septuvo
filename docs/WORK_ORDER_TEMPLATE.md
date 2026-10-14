# Septuvo — universal single-task work order and handoff template

For **Codex, Claude Code, Dex or any other agent**. Copy this template for **every base, dotted subtask, or owner-approved H task**; consult [STATUS.md](STATUS.md), and obey [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md). If owner scope authorization, predecessor acceptance or required gate is missing, **STOP**. A template is not approval.

## Work order

- **Task ID / milestone**: M?-?? / M?-??.? / H-??? (registered in STATUS.md)
- **Predecessor task accepted? Evidence link**:
- **Status** (choose one): `NOT STARTED` | `READY` | `IN PROGRESS` | `BLOCKED` | `IN REVIEW` | `ACCEPTED` | `REJECTED / REWORK`
- **Owner approval: permalink of the owner's `APPROVED WORK ORDER <ID>` comment, its UTC date, and the STATUS.md ledger entry**:
- **Required gate**: G? (evidence/link; or “not applicable — documentation-only”)
- **Objective (one sentence)**:
- **In scope (exact paths and behavior)**:
- **Out of scope / must not modify**:
- **Starting branch / HEAD**:
- **Allowed environments**: (lab only unless explicitly approved)
- **Required design documents and decisions**:
- **Dependencies permitted / prohibited**:
- **Security/reliability risks**:
- **Precise implementation steps (bounded)**:
- **Acceptance criteria (measurable; task catalog cross-reference)**:
- **Security/reliability and negative test cases**:
- **Tests to execute, target OS/tool versions and expected outcomes**:
- **Relevant unaffected regression tests**:
- **Rollback/recovery plan**:
- **Explicit stop point**:

## Completion and handoff

- **Actual branch, complete commit SHA, PR link and current task status**:
- **Files changed**:
- **Commands executed, exit codes, OS and actual outputs/artifacts**:
- **Acceptance criterion PASS / FAIL / NOT RUN with evidence**:
- **Security and privacy impact**:
- **Unresolved blockers / exact decisions needed**:
- **Rollback tested? What and where?**:
- **Actual required gate and owner-recorded state in STATUS.md**:
- **Owner acceptance: permalink of the owner's `ACCEPTED <ID>` comment (if any; never agent-approved)**:
- **One next proposed task only (not started)**:
- **STOP — await owner review before next milestone**:

Do not mark anything PASS when untested. Never infer approval from previous assistant statements or an incomplete handoff.
