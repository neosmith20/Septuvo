"""Shared ledger rules for STATUS.md (D-031). Standard library only.

Used by check_repository.py (offline structure), verify_ledger.py (online
evidence) and sync_status.py (agent-run reconciliation), so all three apply
the same approval model:

  Category A  routine work (H-xxx): no approval needed; the owner's merge of a
              pull request that delivers the task is the acceptance record.
  Category B  milestone work (Mx-yy, Nx-yy, dotted subtasks): needs an owner
              authorization (an owner comment in ordinary language, or the
              owner's merge of a pull request that adds the work order); the
              owner's merge of the delivering pull request accepts it.
  Category C  gates (G0-G4): exact owner comment `GATE Gn PASSED/FAILED`.
              Milestones cannot advance past an unpassed gate, and candidate
              cryptography cannot be marked decided before G1.

A pull request "delivers" a task when the task ID is the scope of one of its
commit subjects (`feat(N1-03): ...`, `docs(M1-01, M1-02): ...`) or of its
title. Mentions elsewhere (for example "next: N1-03") never count.
"""

import re

REPO = "neosmith20/Septuvo"
ROUTINE = "Routine (D-031)"

FENCE = re.compile(r"^\s*(```|~~~)")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
COMMENT_LINK = re.compile(
    r"https://github\.com/(?P<repo>[\w.-]+/[\w.-]+)/(?:issues|pull)/(?P<num>\d+)#issuecomment-(?P<cid>\d+)"
)
PR_LINK = re.compile(r"https://github\.com/(?P<repo>[\w.-]+/[\w.-]+)/pull/(?P<num>\d+)(?![\d#])")
TASK_ID = r"(?:M\d+-\d+(?:\.\d+)?|N\d+-\d+|H-\d{3})"

TASK_STATES = {"NOT STARTED", "READY", "IN PROGRESS", "BLOCKED", "IN REVIEW", "ACCEPTED", "REJECTED / REWORK"}
GATE_STATES = {"PENDING", "PASSED", "FAILED"}
NEEDS_AUTHORIZATION = {"READY", "IN PROGRESS", "IN REVIEW", "ACCEPTED", "REJECTED / REWORK"}
DECIDED = {"ACCEPTED", "REJECTED / REWORK"}

# Gate that must be PASSED before a milestone's tasks may be authorized or worked on
# (ROADMAP.md: STOP until G0 / G1 / G2 / G3).
REQUIRED_GATE = {
    "M1": "G0", "M2": "G0", "N1": "G0", "N2": "G0", "N3": "G0", "N4": "G0",
    "M3": "G1", "M4": "G1", "M5": "G1",
    "M6": "G2", "M7": "G2", "M8": "G2",
    "M9": "G3",
}
# Candidate cryptography (DECISIONS.md) that may not be marked decided before G1.
CRYPTO_DECISIONS = ("D-010", "D-011", "D-012", "D-013", "D-024")

# Rows recorded by the owner directly before D-025 (2026-10-14); kept verbatim.
# Each entry: (state, date, approval_substring, evidence_substring)
LEGACY_BASELINE = {
    "H-001": ("ACCEPTED", "2026-10-14", "Retroactive owner acceptance 2026-10-14", "Documentation workflow fixes: [PR #4]"),
    "H-002": ("ACCEPTED", "2026-10-14", "Owner instruction 2026-10-14: record D-021 option C", "[PR #5]"),
    "H-003": ("ACCEPTED", "2026-10-14", "Retroactive owner acceptance 2026-10-14", "Brand kit corrections: guidelines PDF"),
    "H-004": ("ACCEPTED", "2026-10-14", "Owner instruction 2026-10-14: document public repository", "[PR #7]"),
    "M0-01": ("ACCEPTED", "2026-10-14", "[Approval on issue #9]", "[PR #12]"),
    "M0-01.1": ("ACCEPTED", "2026-10-14", "[Approval on issue #13]", "[PR #15]"),
}


def ledger_rows(text):
    """Yield (kind, ident, state, date, approval, evidence) for each ledger row.

    Rows inside fenced code blocks are ignored, so examples cannot pose as entries.
    """
    in_fence = False
    for line in text.splitlines():
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence or not line.startswith("| "):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        ident = cells[0]
        if re.fullmatch(r"G[0-4]", ident) and len(cells) == 4:
            yield "gate", ident, cells[1], cells[2], "", cells[3]
        elif re.fullmatch(r"M\d+-\d+\.\d+", ident) and len(cells) == 6:
            yield "task", ident, cells[2], cells[3], cells[4], cells[5]
        elif re.fullmatch(r"(?:M|N)\d+-\d+", ident) and len(cells) == 6:
            yield "task", ident, cells[1], cells[3], cells[4], cells[5]
        elif re.fullmatch(r"H-\d{3}", ident) and len(cells) == 5:
            yield "task", ident, cells[1], cells[2], cells[3], cells[4]


def milestone_of(ident):
    """M1-02 -> M1, M0-01.1 -> M0, N1-03 -> N1; H tasks and gates -> None."""
    m = re.match(r"^((?:M|N)\d+)-\d+", ident)
    return m.group(1) if m else None


def category(ident):
    if re.fullmatch(r"G[0-4]", ident):
        return "C"
    return "A" if ident.startswith("H-") else "B"


def mentions(text, ident):
    """True if `ident` appears as a whole token in `text` (M1 does not match M1-01)."""
    return re.search(r"(?<![\w.-])" + re.escape(ident) + r"(?![\w-]|\.\d)", text or "", re.IGNORECASE) is not None


def delivered_ids(subject):
    """Task IDs named in the scope of a conventional subject: `feat(N1-01, N1-02): ...`."""
    m = re.match(r"^\s*[A-Za-z]+\(([^)]*)\)!?\s*:", subject or "")
    if not m:
        m = re.match(r"^\s*(" + TASK_ID + r"(?:\s*[,&+/]\s*" + TASK_ID + r")*)\s*[:—-]", subject or "")
    if not m:
        return set()
    return set(re.findall(TASK_ID, m.group(1)))


def links(cell, pattern):
    return [m for m in pattern.finditer(cell or "") if m.group("repo").lower() == REPO.lower()]


def has_comment_link(cell):
    return bool(links(cell, COMMENT_LINK))


def has_pr_link(cell):
    return bool(links(cell, PR_LINK))


def gate_states(rows):
    return {ident: state for kind, ident, state, *_ in rows if kind == "gate"}


def offline_errors(ledger_text, decisions_text=None):
    """Structural rules that need no network access."""
    errors = []
    rows = list(ledger_rows(ledger_text))
    gates = gate_states(rows)
    for gate in ("G0", "G1", "G2", "G3", "G4"):
        if gate not in gates:
            errors.append(f"STATUS.md: gate row missing or malformed: {gate}")
    for kind, ident, state, date, approval, evidence in rows:
        where = f"STATUS.md {ident}"
        if kind == "gate":
            if state not in GATE_STATES:
                errors.append(f"{where}: invalid gate state {state!r}")
            elif state != "PENDING":
                if not DATE.match(date):
                    errors.append(f"{where}: gate decision needs a YYYY-MM-DD date")
                if not has_comment_link(evidence):
                    errors.append(f"{where}: gate decision needs the owner's `GATE {ident} {state}` comment link")
            continue
        if state not in TASK_STATES:
            errors.append(f"{where}: invalid task state {state!r}")
            continue
        if ident in LEGACY_BASELINE:
            exp_state, exp_date, exp_approval, exp_evidence = LEGACY_BASELINE[ident]
            if state != exp_state or date != exp_date or exp_approval not in approval or exp_evidence not in evidence:
                errors.append(f"{where}: legacy row differs from its recorded baseline")
            continue
        cat = category(ident)
        if state in NEEDS_AUTHORIZATION and not DATE.match(date):
            errors.append(f"{where}: state {state} needs a YYYY-MM-DD date")
        if cat == "B" and state in NEEDS_AUTHORIZATION:
            if not (has_comment_link(approval) or has_pr_link(approval)):
                errors.append(f"{where}: milestone work needs an owner authorization link (comment or merged work-order pull request)")
            required = REQUIRED_GATE.get(milestone_of(ident) or "")
            if required and gates.get(required) != "PASSED":
                errors.append(f"{where}: state {state} requires gate {required} PASSED (it is {gates.get(required, 'missing')})")
        if state in DECIDED and not (has_comment_link(evidence) or has_pr_link(evidence)):
            errors.append(f"{where}: state {state} needs a merged pull request or owner comment link as evidence")
    if decisions_text is not None:
        errors.extend(crypto_decision_errors(decisions_text, gates))
    return errors


def crypto_decision_errors(decisions_text, gates):
    """Candidate cryptography stays Candidate/Blocked until G1 has passed."""
    errors = []
    for line in decisions_text.splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 4 or cells[0] not in CRYPTO_DECISIONS:
            continue
        state = cells[3].lower()
        decided = "owner decided" in state or ("approved" in state and "not approved" not in state)
        if decided and gates.get("G1") != "PASSED":
            errors.append(f"DECISIONS.md {cells[0]}: cryptographic choice marked decided before gate G1 passed")
    return errors
