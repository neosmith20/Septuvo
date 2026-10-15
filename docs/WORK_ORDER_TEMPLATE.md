# Septuvo — universal single-task work order and handoff template

For **Codex, Claude Code, Dex or any other agent**. Use the work order for **milestone work** (category B, D-031): a milestone, a base task or a dotted subtask. Routine H work (category A) needs only the **Completion and handoff** part in its pull request. Consult [STATUS.md](STATUS.md) and obey [AGENT_OPERATIONS.md](AGENT_OPERATIONS.md). If the owner's milestone authorization, predecessor completion or required gate is missing, **STOP**. A template is not approval.

## Work order

- **Task ID / milestone**: M?-?? / M?-??.? / N1-?? / H-??? / a whole milestone such as M1 or N1 (registered in STATUS.md; D-030)
- **Predecessor complete? Merged pull request link**:
- **Status** (choose one): `NOT STARTED` | `READY` | `IN PROGRESS` | `BLOCKED` | `IN REVIEW` | `ACCEPTED` | `REJECTED / REWORK`
- **Owner authorization (category B): link to the owner's comment in ordinary words, or to the owner-merged pull request that added this work order**:
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
- **Owner acceptance: the owner's merge of this pull request (recorded by `sync_status.py`; never agent-approved)**:
- **One next proposed task only (not started)**:
- **STOP — await the owner's review and merge** (the agent opened the PR itself; after the merge it continues with the next task of the same authorized milestone, D-034):

Do not mark anything PASS when untested. Never infer approval from previous assistant statements or an incomplete handoff.
