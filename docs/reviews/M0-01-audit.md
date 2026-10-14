# M0-01 — Planning documents audit

| | |
| --- | --- |
| **Task** | M0-01 — Audit existing planning documents ([TASK_CATALOG.md](../TASK_CATALOG.md#m0-01--audit-existing-planning-documents)) |
| **Work order** | [Issue #9](https://github.com/neosmith20/Septuvo/issues/9), approved by the owner on 2026-10-14 (comment `APPROVED WORK ORDER M0-01`); ledger state READY in [STATUS.md](../STATUS.md) |
| **Audited revision** | `main` @ `98fc630a866cf4750aac9a542e4f0d5d32c3eb58`. The work order named `b714265`; the only change since is the M0-01 ledger row in STATUS.md, which was re-read. |
| **Auditor** | Claude (AI agent). This is an agent-produced documentation review, **not** an independent audit and not a security assessment. |
| **Status** | IN REVIEW — owner decides acceptance in STATUS.md |

## Summary

The planning set is broadly coherent: task IDs, order and the ledger agree; gates G0–G4 are described consistently; S-01…S-13 are all traced; no candidate cryptography is presented as approved; no relative link is broken. The audit found **20 findings: 0 Critical, 1 High, 5 Medium, 14 Low**. The High and most Medium findings are about the *approval process itself* (what counts as owner approval, who records task progress, and how a work-order revision is identified), not about the VPN design. Six findings need an owner decision before they can be corrected (see [Owner decisions needed](#owner-decisions-needed)). All findings are **OPEN**; none were fixed in this task.

## Disclosure of auditor involvement

The same agent that performed this audit wrote parts of the audited material earlier on 2026-10-14: the H-002 changes ([PR #5](https://github.com/neosmith20/Septuvo/pull/5): D-021 wording, catalog predecessors, work-order status line), the H-004 changes ([PR #7](https://github.com/neosmith20/Septuvo/pull/7): `docs/security/GITHUB_SECURITY_SETUP.md`, the SECURITY.md reporting section, D-022 and D-023), the text of the H-001…H-004 ledger rows that the owner pasted into STATUS.md, and the M0-01 work order itself. Findings A-01, A-04 and A-16 concern that material. Everything was re-checked from the files at the audited revision rather than from memory, but a second reviewer would be less likely to share the author's blind spots.

## Scope and method

**Files audited (read in full):** `README.md`, `LICENSE`, `AGENTS.md`, `SECURITY.md`, `.github/PULL_REQUEST_TEMPLATE.md`, `.github/workflows/repository-integrity.yml`, `.github/dependabot.yml`, `scripts/check_repository.py`, and `docs/`: `ROADMAP.md`, `TASK_CATALOG.md`, `STATUS.md`, `AGENT_OPERATIONS.md`, `WORK_ORDER_TEMPLATE.md`, `ARCHITECTURE.md`, `SECURITY_MODEL.md`, `DECISIONS.md`, `TRACEABILITY.md`, `TEST_MATRIX.md`, `PROTOCOL_REVIEW_CHECKLIST.md`, `security/GITHUB_SECURITY_SETUP.md`, `security/MAIN_RULESET.json`. `brand/` was out of scope (accepted under H-003) except that its Markdown links were included in the link check.

**Environment:** isolated cloud workspace, Linux 6.18, Python 3.13.16, read-only clone of the repository. Helper scripts used for the cross-checks ran in the workspace and are not committed.

**Severity scale used in this report**

| Severity | Meaning |
| --- | --- |
| Critical | Could lead to invented or unreviewed cryptography, security-relevant implementation before its gate, or a gate being passed without the required evidence |
| High | Contradiction in the rules that could plausibly let an agent act without valid owner authorization |
| Medium | Inconsistency that leaves a process, decision or crypto state ambiguous; should be resolved before G0 |
| Low | Wording, naming, completeness or tooling issue that does not change what an agent is allowed to do |

## Check results

| # | Check | Method | Result |
| --- | --- | --- | --- |
| — | Repository integrity script | `python3 scripts/check_repository.py` | **PASS** — "16 Markdown files inspected, 43 ordered tasks, 13 security requirements, G0-G4 present" (exit 0) |
| a | Task IDs, order, predecessors | Compared catalog headings, catalog "Fixed task order", ROADMAP bullets and STATUS ledger; every `M?-??` reference in every Markdown file; ledger predecessor vs catalog *Entry / prerequisites* | **PASS with findings** — 43 IDs, identical order in all four places, no unknown IDs. One card omits its ledger predecessor (A-09); the two M0-01 scope lists differ (A-08) |
| b | Gate definitions G0–G4 | Compared AGENTS.md *Gate policy*, ROADMAP *Gate summary*, SECURITY_MODEL *Security gates / evidence*, STATUS gate table, AGENT_OPERATIONS *Stage and release boundaries*; listed gate prerequisites in task cards | **PASS with findings** — definitions consistent; gate prerequisites present at M1-01/M2-01 (G0), M3-01 (G1), M6-01 (G2), M9-01 (G3). Vocabulary drift in A-12 |
| c | Security requirements S-01…S-13 | Presence in SECURITY_MODEL and TRACEABILITY; search for S-14+; T-categories used vs defined | **PASS with finding** — all 13 present and traced, none undefined. T0 and T4 never mapped (A-13) |
| d | Decision IDs D-001…D-023 | Defined IDs contiguous; every `D-???` reference resolves; state labels compared | **PASS with findings** — 23 contiguous IDs, no undefined references. Approval attribution and state labels inconsistent (A-06); D-023 conflicts with the approval procedure (A-01) |
| e | Product names (D-001) | Searched all in-scope Markdown for variant names | **PASS with finding** — the four official names are used correctly; "SVP protocol" redundancy in four places (A-11) |
| f | Links and anchors | Every relative Markdown link and `#anchor` in all repository Markdown including `brand/`; GitHub URLs checked through the GitHub API | **PASS** — 0 broken relative links or anchors; 7 GitHub URLs (PR #4–#7, issue #9, commits `160fbde`, `c3195eb`) resolve. **NOT RUN:** 5 non-GitHub URLs (NIST FIPS 203, FIPS 204, SP 800-38D; RFC 10024; RFC 5869) — not reachable from the workspace |
| g | Cryptography and claims | Every mention of AES-GCM, ChaCha20, X25519, ML-KEM, ML-DSA, HKDF/KDF checked against D-010…D-013; searched for audit, production, PQ-proof, certified, guarantee wording | **PASS with findings** — no candidate presented as approved and no unqualified audit/production/PQ claim. KDF state ambiguous (A-05); some requirement text presumes candidates (A-07); README wording (A-10) |
| h | Task-state vocabulary and handoff format | Compared state lists in AGENTS, AGENT_OPERATIONS, STATUS, WORK_ORDER_TEMPLATE, PR template; traced handoff references | **PASS with findings** — one canonical handoff format, referenced consistently. Process gap in A-03; vocabulary drift in A-12 |
| i | README claims vs repository contents | Each README statement compared with what the repository contains | **PASS with finding** — status disclaimer accurate; two present-tense phrases (A-10) |
| — | LICENSE | Read header and body | **PASS** — complete Apache License 2.0 text, matching README and the catalog |

## Findings overview

| ID | Severity | Location | Finding |
| --- | --- | --- | --- |
| A-01 | **High** | DECISIONS.md D-023 | Defines owner approval as "merge plus the instruction recorded in the PR", conflicting with the formal approval procedure |
| A-02 | Medium | AGENTS.md, AGENT_OPERATIONS.md, ROADMAP.md, DECISIONS.md D-023 | Four different rules on pushing to `main` |
| A-03 | Medium | AGENT_OPERATIONS.md dispatch, STATUS.md | Dispatch relies on IN PROGRESS / IN REVIEW states that nobody is assigned to record |
| A-04 | Medium | STATUS.md approval procedure and M0-01 row | Procedure requires a link to the exact work-order revision; issue-based approvals have no revision permalink |
| A-05 | Medium | ROADMAP M2-02, TASK_CATALOG invariant 3, D-013, P-05, README | HKDF is treated as the KDF in several places but has no candidate or decision entry |
| A-06 | Medium | DECISIONS.md D-001…D-020 | No dates or owner attribution; 12 different state labels |
| A-07 | Low | PROTOCOL_REVIEW_CHECKLIST P-04, SECURITY_MODEL S-04, TEST_MATRIX T1 | Requirement and test text names candidate primitives as if selected |
| A-08 | Low | TASK_CATALOG M0-01, ROADMAP M0-01 | The two documents list different files for this audit |
| A-09 | Low | TASK_CATALOG M9-01 | Entry does not name its ledger predecessor M8-04 |
| A-10 | Low | README.md | Secondary tagline not in D-002; "openly specified" while no specification exists |
| A-11 | Low | SECURITY.md, ARCHITECTURE.md, ROADMAP.md, TASK_CATALOG.md | "SVP protocol" (Septuvo Protocol protocol) instead of D-001 naming |
| A-12 | Low | TASK_CATALOG M5-05, M7-05 | "PASS/FAIL" and "UNSUPPORTED" outside the defined vocabularies |
| A-13 | Low | TRACEABILITY.md | Test categories T0 and T4 are never mapped |
| A-14 | Low | STATUS.md header | "All 43 base tasks are NOT STARTED" is no longer true |
| A-15 | Low | AGENT_OPERATIONS *Stage and release boundaries* | "Before G0: inspection and proposal only" omits permitted pre-G0 edits |
| A-16 | Low | STATUS.md H-002 and H-004 rows | Approvals cite chat instructions, not permalinks |
| A-17 | Low | SECURITY.md | Calls G3 a "GitHub release approval gate" |
| A-18 | Low | ROADMAP M0 acceptance | "Every 'must' has a validation location" is not assigned to any task |
| A-19 | Low | scripts/check_repository.py | Coverage gaps in the integrity check |
| A-20 | Low | .github/dependabot.yml | Cargo ecosystem configured before any manifest exists |

## Findings in detail

### A-01 — D-023 redefines owner approval (High)

- **Location:** [DECISIONS.md](../DECISIONS.md) line 29, D-023; conflicts with [AGENT_OPERATIONS.md](../AGENT_OPERATIONS.md) *What constitutes approval* (line 20) and [STATUS.md](../STATUS.md) *Work-order and task approval procedure* step 2 (line 10).
- **Evidence:** D-023: "owner approval is the owner's merge plus the instruction recorded in the PR." AGENT_OPERATIONS: "A PR, issue, comment by another agent, merge or green CI alone is not authorization." STATUS: "Owner comments `APPROVED WORK ORDER <ID>` … A PR, merge or CI pass alone is not approval."
- **Why it matters:** the PR description is written by the agent. Read literally, D-023 lets an agent record an "instruction" in its own PR text and treat the owner's merge as approval of it, which is the outcome the other two documents exist to prevent. The two procedures also produce different evidence (merge vs. comment + ledger).
- **Proposed correction:** narrow D-023 to account access only, and state that approval is defined solely by STATUS.md step 2 (owner-authored `APPROVED WORK ORDER` comment plus owner ledger entry); a merge accepts the reviewed change but is never approval of new scope, and text written by an agent is never evidence of an owner instruction. *Needs owner decision.*
- **Status:** OPEN

### A-02 — Inconsistent rules for pushing to `main` (Medium)

- **Location:** [AGENTS.md](../../AGENTS.md) *Start-of-task checklist* item 5 (line 19); [AGENT_OPERATIONS.md](../AGENT_OPERATIONS.md) *Start here* item 5 (line 13); [ROADMAP.md](../ROADMAP.md) *Execution sequence and discipline* (line 114); [DECISIONS.md](../DECISIONS.md) D-023 (line 29); [GITHUB_SECURITY_SETUP.md](../security/GITHUB_SECURITY_SETUP.md) *Active controls on `main`*.
- **Evidence:** AGENTS: "do not push directly to main unless the owner explicitly directs it." AGENT_OPERATIONS: "Leave main untouched; propose changes through a PR." ROADMAP: "Branch + PR for code" (implies non-code may not need a PR). D-023: "Agents never merge, push to `main` …". The active ruleset blocks direct pushes for everyone with no bypass.
- **Why it matters:** an agent reading AGENTS.md alone would believe a direct push is allowed on owner instruction; that is now impossible and contradicts D-023. The ROADMAP wording suggests documentation changes could skip a PR.
- **Proposed correction:** use one sentence in all three rule documents: "Every change, code or documentation, goes through a task branch and pull request; agents never push to, merge into, or force-push `main`." Reference the ruleset in GITHUB_SECURITY_SETUP.md.
- **Status:** OPEN

### A-03 — Nobody records IN PROGRESS or IN REVIEW (Medium)

- **Location:** [AGENT_OPERATIONS.md](../AGENT_OPERATIONS.md) *Deterministic dispatch algorithm* (lines 44–60); [STATUS.md](../STATUS.md) *OWNER ONLY* (line 5) and procedure steps 3–4 (lines 11–12).
- **Evidence:** the algorithm branches on ledger states: "if IN REVIEW: wait for owner; STOP" and "if IN PROGRESS: reconcile actual branch/PR". STATUS.md: "Agents must not edit this file", and the procedure only has the owner record READY, ACCEPTED or REJECTED / REWORK.
- **Why it matters:** after an agent starts or submits work, the ledger still shows READY. A second agent (or the same one after an interruption) following the algorithm would see READY and could start the same task again or treat it as not yet submitted.
- **Proposed correction (choose one):** (a) the owner also records IN PROGRESS / IN REVIEW; or (b) keep the ledger to owner decisions only and add a dispatch step: "before starting a READY task, search open pull requests and branches for its ID; if one exists, treat the task as IN PROGRESS / IN REVIEW." Option (b) adds no owner workload. *Needs owner decision.*
- **Status:** OPEN

### A-04 — Work-order revision cannot be pinned for issue-based approvals (Medium)

- **Location:** [STATUS.md](../STATUS.md) procedure step 2 (line 10), dates/permalinks rule (line 16), and the M0-01 row (line 32).
- **Evidence:** step 2 requires "a link to the exact work-order revision in a GitHub issue/PR" and the ledger to hold "the comment/work-order links". The M0-01 row links `https://github.com/neosmith20/Septuvo/issues/9` rather than the approval comment, and a GitHub issue description has no permanent per-revision link (only edit history).
- **Why it matters:** this applies to the approval of this very task. If an issue description is edited after approval, the ledger cannot show which version was approved.
- **Proposed correction:** define the approved revision as "the issue or PR description as it stood when the owner's approval comment was posted, and the ledger links the approval comment's permalink" (for M0-01: `https://github.com/neosmith20/Septuvo/issues/9#issuecomment-6161523935`). Alternatively, keep work orders as files in a PR so the commit SHA identifies the revision. *Needs owner decision on which form to use.*
- **Status:** OPEN

### A-05 — KDF choice is ambiguous; HKDF has no decision entry (Medium)

- **Location:** [ROADMAP.md](../ROADMAP.md) M2-02 (line 39); [TASK_CATALOG.md](../TASK_CATALOG.md) *Task catalog invariants* item 3 (line 419); [DECISIONS.md](../DECISIONS.md) D-013 (line 19); [PROTOCOL_REVIEW_CHECKLIST.md](../PROTOCOL_REVIEW_CHECKLIST.md) P-05 (line 13); [README.md](../../README.md) *References* (line 71).
- **Evidence:** ROADMAP M2-02 instructs drafting "KEM validation/combiner, HKDF, key confirmation"; TASK_CATALOG calls HKDF one of the "candidate choices"; D-013's area is "HKDF / handshake / AEAD nonces / packet framing" with "Details undecided … BLOCKED"; P-05 asks for "extract/expand steps" (HKDF terminology). No decision row lists HKDF, or any KDF, as a candidate the way D-010…D-012 do for the cipher, KEM and signature.
- **Why it matters:** an agent drafting M2-02 would reasonably treat HKDF as already chosen, which is exactly the "candidate becomes fact" drift DECISIONS.md warns against (line 43).
- **Proposed correction:** either add a decision row "KDF: HKDF (RFC 5869) — Candidate, not approved" alongside D-010…D-012, or make ROADMAP M2-02, the invariant and P-05 KDF-neutral ("the KDF and its extract/expand or equivalent steps"). *Needs owner decision.*
- **Status:** OPEN

### A-06 — Decision states lack attribution and use many labels (Medium)

- **Location:** [DECISIONS.md](../DECISIONS.md) table rows D-001…D-020 (lines 7–26) and the rule at line 43.
- **Evidence:** the rule requires "rationale, rejected alternatives, consequences and owner authorization" for each decision, but D-001…D-020 carry no dates, approver or rejected alternatives, and use 12 distinct state labels: Owner confirmed, Owner confirmed (working copy), Direction agreed, Goal, Candidate (three variants), BLOCKED, Proposed sequencing, Policy direction, OPEN, Unmeasured aspiration, Product requirement. Only D-021…D-023 are dated.
- **Why it matters:** it is unclear which entries the owner actually approved ("Direction agreed" — by whom, when?). G0 requires reviewed decisions.
- **Proposed correction:** define a short state vocabulary (for example: Owner decided / Candidate / Open / Blocked) with a date and approval reference for every "Owner decided" row. Most naturally done in **M0-02**, which already owns `docs/DECISIONS.md`, rather than in M0-01.1.
- **Status:** OPEN

### A-07 — Requirement and test text names candidate primitives (Low)

- **Location:** [PROTOCOL_REVIEW_CHECKLIST.md](../PROTOCOL_REVIEW_CHECKLIST.md) P-04 (line 12); [SECURITY_MODEL.md](../SECURITY_MODEL.md) S-04 (line 37); [TEST_MATRIX.md](../TEST_MATRIX.md) T1 (line 16).
- **Evidence:** P-04: "Ephemeral X25519/ML-KEM roles"; S-04: "cannot reuse an AES-GCM key/nonce combination"; T1: "Corrupt tag/signature/ML-KEM ciphertext". D-010 and D-011 list these as candidates only, and D-010 also allows ChaCha20-Poly1305.
- **Why it matters:** the requirements should hold whichever primitives are finally chosen; naming one candidate implies selection and would leave the requirement incomplete if a different suite were approved.
- **Proposed correction:** phrase generically with the candidate in parentheses, e.g. S-04 "… cannot reuse an AEAD key/nonce combination (e.g. AES-GCM, candidate D-010)"; P-04 "classical and post-quantum KEM roles (candidate D-011: X25519 + ML-KEM-768)"; T1 "corrupt KEM ciphertext".
- **Status:** OPEN

### A-08 — Two different file lists for M0-01 (Low)

- **Location:** [TASK_CATALOG.md](../TASK_CATALOG.md) M0-01 *Exact work* (line 39); [ROADMAP.md](../ROADMAP.md) M0-01 (line 21).
- **Evidence:** catalog: "README, license, AGENTS, roadmap, architecture, security model, decisions, task catalog, and traceability". ROADMAP: "README, license, AGENTS.md, architecture, threat model, decision register, protocol checklist, roadmap and test matrix". The catalog omits the protocol checklist and test matrix; the ROADMAP omits the task catalog and traceability; neither mentions STATUS.md, AGENT_OPERATIONS.md or the work-order template.
- **Why it matters:** the scope of this task depended on which document was read. This audit used the union plus the process files, as approved in the work order.
- **Proposed correction:** make both read "all planning documents listed in the approved work order (at minimum: …)" with one identical list.
- **Status:** OPEN

### A-09 — M9-01 does not name its predecessor (Low)

- **Location:** [TASK_CATALOG.md](../TASK_CATALOG.md) M9-01 *Entry / prerequisites* (line 376); [STATUS.md](../STATUS.md) M9-01 row (line 70).
- **Evidence:** catalog: "G3 passed for applicable core; enterprise scope owner-approved." The ledger lists predecessor M8-04. Every other card names its ledger predecessor (checked by script).
- **Proposed correction:** "M8-04 accepted; G3 passed for applicable core; enterprise scope owner-approved."
- **Status:** OPEN

### A-10 — README present-tense wording (Low)

- **Location:** [README.md](../../README.md) line 5 and *Official product names* (line 17).
- **Evidence:** line 5: "*Private by design. Powerful by nature. Simple for everyone.*" — not recorded in D-002, which lists only "Security Without Compromise." Line 17 describes SVP as an "Independent, openly specified networking protocol", while line 7 states there is no "published SVP wire protocol".
- **Proposed correction:** either record the secondary tagline in D-002 (as aspiration) or remove it; change line 17 to "Independent networking protocol, to be openly specified".
- **Status:** OPEN

### A-11 — "SVP protocol" naming (Low)

- **Location:** [SECURITY.md](../../SECURITY.md) line 5 ("audited SVP protocol"); [ARCHITECTURE.md](../ARCHITECTURE.md) line 20 ("independent SVP protocol state machine"); [ROADMAP.md](../ROADMAP.md) line 36 heading ("formal SVP protocol specification"); [TASK_CATALOG.md](../TASK_CATALOG.md) line 103 heading ("SVP protocol specification").
- **Evidence:** D-001 names it "Septuvo Protocol (SVP)", so "SVP protocol" expands to *Septuvo Protocol protocol*. TRACEABILITY and TEST_MATRIX expect consistent naming ("Four official names", T0).
- **Proposed correction:** use "SVP" or "Septuvo Protocol (SVP)", e.g. "SVP specification", "SVP state machine".
- **Status:** OPEN

### A-12 — State vocabulary drift (Low)

- **Location:** [TASK_CATALOG.md](../TASK_CATALOG.md) M5-05 *Acceptance evidence* (line 255) and M7-05 *Acceptance evidence* (line 333); compare [STATUS.md](../STATUS.md) line 13 and *Allowed task states* (line 97).
- **Evidence:** M5-05: "G2 owner PASS/FAIL recorded explicitly" while gates use PASSED / FAILED. M7-05: "each platform separately ACCEPTED or UNSUPPORTED" — UNSUPPORTED is not a defined task state and ACCEPTED is a task state, not a platform state.
- **Proposed correction:** M5-05 "G2 recorded by the owner as PASSED or FAILED"; M7-05 "each platform recorded as supported or unsupported in the platform matrix".
- **Status:** OPEN

### A-13 — T0 and T4 are not traced (Low)

- **Location:** [TRACEABILITY.md](../TRACEABILITY.md) (all tables); [TEST_MATRIX.md](../TEST_MATRIX.md) T0 (line 5) and T4 (line 39).
- **Evidence:** TRACEABILITY references T1, T2, T3 and T5 only. D-018 (public performance goals) has no traceability row and no row cites T4; the "Four official names" row does not cite T0.
- **Proposed correction:** add T0 to the "Four official names" row and T4 to a performance row (D-018). Can be folded into M1-04, which owns TRACEABILITY.md.
- **Status:** OPEN

### A-14 — STATUS.md header is stale (Low)

- **Location:** [STATUS.md](../STATUS.md) line 3.
- **Evidence:** "Initial baseline only: all 43 base tasks are NOT STARTED and all gates are PENDING." M0-01 is READY and four H tasks are ACCEPTED.
- **Proposed correction (owner-only file):** "Created as a baseline with all tasks NOT STARTED and all gates PENDING; the tables below are the current state."
- **Status:** OPEN

### A-15 — Pre-G0 boundary omits permitted edits (Low)

- **Location:** [AGENT_OPERATIONS.md](../AGENT_OPERATIONS.md) *Stage and release boundaries* (line 102).
- **Evidence:** "Before G0: architecture/documentation inspection and proposal only." Yet M0-01.1 corrects documents, M0-02 writes `docs/DECISIONS.md` (TASK_CATALOG line 54), and H-001…H-004 changed documents and brand assets before G0.
- **Proposed correction:** "Before G0: documentation inspection, owner-approved documentation corrections (M0 tasks and H tasks) and proposals only; no code."
- **Status:** OPEN

### A-16 — H-002 and H-004 approvals cite chat instructions (Low)

- **Location:** [STATUS.md](../STATUS.md) H-002 (line 91) and H-004 (line 93) rows; rule at line 16.
- **Evidence:** both rows record "Owner instruction 2026-10-14: …" as the work-order approval. Those instructions were given in a chat session outside the repository, so there is no permalink, contrary to "Use dates in `YYYY-MM-DD` UTC and permalinks." The completion evidence (PR #5, PR #7) does link correctly.
- **Proposed correction:** no change needed for accepted history if the owner considers the PR merges sufficient; for future H tasks, use the issue-based approval used for M0-01 (subject to A-04).
- **Status:** OPEN

### A-17 — G3 described as a GitHub gate (Low)

- **Location:** [SECURITY.md](../../SECURITY.md) *Release and support rules* (line 23); compare [GITHUB_SECURITY_SETUP.md](../security/GITHUB_SECURITY_SETUP.md) line 63.
- **Evidence:** SECURITY.md: "until GitHub release approval gate **G3**". GITHUB_SECURITY_SETUP.md: "The gates concern the VPN product, not repository settings."
- **Proposed correction:** "until the production release gate **G3** in the engineering roadmap".
- **Status:** OPEN

### A-18 — M0 acceptance item without an owner (Low)

- **Location:** [ROADMAP.md](../ROADMAP.md) M0 *Acceptance* (line 25).
- **Evidence:** "every “must” has a validation location" is a G0 acceptance criterion, but no M0 task card (M0-01, M0-02, M0-03) is asked to produce that mapping, and this audit's approved scope did not include it. TRACEABILITY covers S-01…S-13 and selected product requirements, not every "must" in the documents.
- **Proposed correction:** assign it explicitly (for example to M0-03's G0 evidence package) or reword it to "every security requirement S-01…S-13 and every product requirement in TRACEABILITY.md has a validation location".
- **Status:** OPEN

### A-19 — Integrity script coverage gaps (Low)

- **Location:** [scripts/check_repository.py](../../scripts/check_repository.py) lines 10–28 (`REQUIRED`), 29 (`LINK`), 46–49, 74–76.
- **Evidence:** the link check scans README, AGENTS, SECURITY, `docs/**` and `.github/**` Markdown only, so `brand/README.md` and `brand/04 - Colors And Tokens/brand-tokens.md` are not checked; image links are excluded by the `(?<!!)` pattern; `#anchor` targets are not verified; `docs/security/*` and the PR template are not in `REQUIRED`; lines 46–49 test the same duplicate condition twice. This audit's own link and anchor check found no current breakage, so this is a coverage gap, not a present defect.
- **Proposed correction:** extend the scan to all repository Markdown (excluding `.git`), include image links and anchors, add the missing required files, and remove the duplicate test. Changes to CI tooling need their own work order.
- **Status:** OPEN

### A-20 — Dependabot Cargo entry before a manifest exists (Low)

- **Location:** [.github/dependabot.yml](../../.github/dependabot.yml) line 13.
- **Evidence:** `package-ecosystem: "cargo"` is configured although no `Cargo.toml` exists until M3-01. The file's own comment acknowledges this. Depending on GitHub's behaviour this may show a configuration warning in the Dependabot view; this was not observable from the workspace (NOT RUN).
- **Proposed correction:** no action required; the owner has previously accepted this. Optionally move the Cargo entry into the M3-01 work order.
- **Status:** OPEN

## Owner decisions needed

1. **A-01** — Confirm that only the STATUS.md procedure (owner comment + owner ledger entry) counts as approval, and narrow D-023 accordingly.
2. **A-03** — Choose who reflects IN PROGRESS / IN REVIEW: the owner in the ledger, or agents by checking open PRs and branches.
3. **A-04** — Choose how a work-order revision is identified: approval-comment permalink on an issue, or work orders committed as files.
4. **A-05** — Record HKDF as a KDF candidate in DECISIONS.md, or make the roadmap, invariant and P-05 KDF-neutral.
5. **A-06** — Agree a decision-state vocabulary and date/approval convention (proposed for M0-02).
6. **A-10** — Keep (and record) or remove the README secondary tagline.

## Not covered by this audit

- Non-GitHub external links (5) — NOT RUN, unreachable from the workspace.
- Brand assets and brand documents beyond their links — out of scope (H-003).
- GitHub repository settings — not audited here; their documented state is in GITHUB_SECURITY_SETUP.md.
- Whether every "must" in the documents has a validation location — not in this task's scope (see A-18).

## Recommended next step

Owner reviews this report and records M0-01 as ACCEPTED or REJECTED / REWORK. Because findings exist, the catalog requires **M0-01.1** (corrections) before M0-02. Suggested split: M0-01.1 corrects A-02 and A-07…A-19 (wording and tooling; A-14 and any STATUS.md change remain owner-only) plus A-01, A-03, A-04, A-05 once decided; A-06 is handled in M0-02. M0-01.1 needs its own owner-approved work order. This agent has not started it.
