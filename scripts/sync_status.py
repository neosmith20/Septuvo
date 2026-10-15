#!/usr/bin/env python3
"""Reconcile docs/STATUS.md with GitHub so the owner never edits ledger tables (D-031).

Reads pull requests merged into main by the owner and, for every task they
deliver (task ID in a commit-subject or title scope, see ledger_rules.py):

- routine tasks (H-xxx): marks the row ACCEPTED with the merge date and the
  pull request link, adding the row if it is missing;
- milestone tasks (Mx-yy, Nx-yy): marks the row ACCEPTED only if the row
  already carries an owner authorization; otherwise reports that an owner
  decision is needed and changes nothing.

It never records approvals, gate decisions or cryptographic decisions, never
changes the recorded legacy rows, and never marks work done that no owner-merged
pull request delivered. Every row it writes passes verify_ledger.py.

Usage:
  sync_status.py            report drift only (exit 0); used by CI as a notice
  sync_status.py --write    update docs/STATUS.md in place (agents, in a pull request)
"""

import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ledger_rules as L  # noqa: E402
import verify_ledger as V  # noqa: E402

ROOT = V.ROOT
PR_URL = "https://github.com/" + L.REPO + "/pull/{}"


def merged_pulls(gh, owner, pages=3):
    """Owner-merged pull requests into main, newest first."""
    out = []
    for page in range(1, pages + 1):
        batch = gh.get(f"/repos/{gh.repo}/pulls", {"state": "closed", "base": "main",
                                                   "per_page": 100, "page": page}) or []
        for pr in batch:
            if pr.get("merged_at"):
                full = gh.pull(pr["number"])
                if V.merged_by_owner(gh, full, owner):
                    out.append(full)
        if len(batch) < 100:
            break
    return out


def deliveries(gh, owner):
    """{task_id: [(merge_date, pr_number), ...]} from owner-merged pull requests."""
    found = {}
    for pr in merged_pulls(gh, owner):
        for ident in gh.delivered(pr["number"]):
            found.setdefault(ident, []).append((pr["merged_at"][:10], pr["number"]))
    return found


def evidence_cell(prs):
    return "; ".join(f"[PR #{n}]({PR_URL.format(n)})" for _, n in sorted(prs, key=lambda p: p[1]))


def plan(ledger_text, found):
    """Return (row_updates, new_h_rows, notes). Pure function, unit-tested."""
    updates, notes = {}, []
    rows = {ident: (kind, state, date, approval, evidence)
            for kind, ident, state, date, approval, evidence in L.ledger_rows(ledger_text)}
    for ident, prs in sorted(found.items()):
        row = rows.get(ident)
        last = max(d for d, _ in prs)
        if row is None:
            if L.category(ident) == "A":
                continue  # added below as a new H row
            notes.append(f"{ident}: delivered by {evidence_cell(prs)} but has no ledger row; owner decision needed")
            continue
        kind, state, date, approval, evidence = row
        if ident in L.LEGACY_BASELINE or state in L.DECIDED:
            continue
        if L.category(ident) == "B" and not (L.has_comment_link(approval) or L.has_pr_link(approval)):
            notes.append(f"{ident}: merged in {evidence_cell(prs)} without a recorded owner authorization; owner decision needed")
            continue
        updates[ident] = ("ACCEPTED", last, approval if approval not in {"", "—", "-"} else L.ROUTINE, evidence_cell(prs))
        notes.append(f"{ident}: {state} -> ACCEPTED ({evidence_cell(prs)}, merged {last})")
    new_h = []
    for ident, prs in sorted(found.items()):
        if L.category(ident) == "A" and ident not in rows:
            last = max(d for d, _ in prs)
            new_h.append((ident, last, evidence_cell(prs)))
            notes.append(f"{ident}: new routine row, ACCEPTED ({evidence_cell(prs)}, merged {last})")
    return updates, new_h, notes


def apply(ledger_text, updates, new_h):
    """Rewrite the affected table rows; append new H rows to the hygiene table."""
    out, in_fence, last_h_index = [], False, None
    for line in ledger_text.splitlines():
        if L.FENCE.match(line):
            in_fence = not in_fence
        if not in_fence and line.startswith("| "):
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            ident = cells[0]
            if ident in updates:
                state, date, approval, evidence = updates[ident]
                if re.fullmatch(r"M\d+-\d+\.\d+", ident) and len(cells) == 6:
                    cells = [ident, cells[1], state, date, approval, evidence]
                elif len(cells) == 6:
                    cells = [ident, state, cells[2], date, approval, evidence]
                elif len(cells) == 5:
                    cells = [ident, state, date, approval, evidence]
                line = "| " + " | ".join(cells) + " |"
            if re.fullmatch(r"H-\d{3}", ident) and len(cells) == 5:
                last_h_index = len(out)
        out.append(line)
    if new_h:
        if last_h_index is None:
            raise SystemExit("ERROR: hygiene table (H rows) not found in STATUS.md")
        rows = [f"| {i} | ACCEPTED | {d} | {L.ROUTINE} | {e} |" for i, d, e in new_h]
        out[last_h_index + 1:last_h_index + 1] = rows
    return "\n".join(out) + "\n"


def main(argv):
    write = "--write" in argv
    path = ROOT / "docs/STATUS.md"
    text = path.read_text(encoding="utf-8")
    gh = V.GitHub()
    owner = gh.owner()
    updates, new_h, notes = plan(text, deliveries(gh, owner))
    if not notes:
        print("STATUS.md is in sync with owner-merged pull requests.")
        return 0
    for note in notes:
        print(("::notice title=Ledger drift::" if os.environ.get("GITHUB_ACTIONS") else "") + note)
    if write and (updates or new_h):
        path.write_text(apply(text, updates, new_h), encoding="utf-8")
        print(f"Updated {path.relative_to(ROOT)}: {len(updates)} row(s) changed, {len(new_h)} row(s) added.")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except V.Unverifiable as exc:
        print(f"ERROR: could not read GitHub: {exc}", file=sys.stderr)
        sys.exit(2)
