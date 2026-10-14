#!/usr/bin/env python3
"""Standard-library-only documentation integrity check. Not a security audit.

Offline checks: required files; task IDs and order; ledger structure and
decision-comment links (D-025); security-requirement tracing; gates; and every
relative Markdown link, image and #anchor in the repository.
Online verification of the linked owner comments is done by verify_ledger.py.
"""

from pathlib import Path
from urllib.parse import unquote
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = (
    "README.md",
    "AGENTS.md",
    "LICENSE",
    "SECURITY.md",
    "docs/ROADMAP.md",
    "docs/STATUS.md",
    "docs/TASK_CATALOG.md",
    "docs/AGENT_OPERATIONS.md",
    "docs/ARCHITECTURE.md",
    "docs/SECURITY_MODEL.md",
    "docs/TRACEABILITY.md",
    "docs/WORK_ORDER_TEMPLATE.md",
    "docs/TEST_MATRIX.md",
    "docs/DECISIONS.md",
    "docs/PROTOCOL_REVIEW_CHECKLIST.md",
    "docs/security/GITHUB_SECURITY_SETUP.md",
    "docs/security/MAIN_RULESET.json",
    ".github/dependabot.yml",
    ".github/workflows/repository-integrity.yml",
    ".github/PULL_REQUEST_TEMPLATE.md",
    "scripts/verify_ledger.py",
)
LINK = re.compile(r"!?\[[^\]\n]*\]\(([^)\n]+)\)")
INLINE_CODE = re.compile(r"`[^`\n]*`")
FENCE = re.compile(r"^\s*(```|~~~)")
HEADING = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$")
CATALOG_ID = re.compile(r"^### (M\d+-\d+) —", re.MULTILINE)
ROADMAP_ID = re.compile(r"\*\*(M\d+-\d+):\*\*")
SUBTASK_ID = re.compile(r"^### (M\d+-\d+\.\d+) —", re.MULTILINE)
VALID_WORK_ID = re.compile(r"^(?:M\d+-\d+(?:\.\d+)?|H-\d{3})$")
PERMALINK = re.compile(r"https://github\.com/neosmith20/Septuvo/(?:issues|pull)/\d+#issuecomment-\d+")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
TASK_STATES = {"NOT STARTED", "READY", "IN PROGRESS", "BLOCKED", "IN REVIEW", "ACCEPTED", "REJECTED / REWORK"}
GATE_STATES = {"PENDING", "PASSED", "FAILED"}
NEEDS_APPROVAL = {"READY", "IN PROGRESS", "IN REVIEW", "ACCEPTED", "REJECTED / REWORK"}
# Rows recorded by the owner directly before D-025 (2026-10-14).
LEGACY_APPROVAL = {"H-001", "H-002", "H-003", "H-004"}
LEGACY_DECISION = LEGACY_APPROVAL | {"M0-01", "M0-01.1"}

errors = []


def ledger_rows(text):
    """Yield (kind, id, state, date, approval, evidence) for every ledger table row."""
    for line in text.splitlines():
        if not line.startswith("| "):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        ident = cells[0]
        if re.fullmatch(r"G[0-4]", ident) and len(cells) == 4:
            yield "gate", ident, cells[1], cells[2], "", cells[3]
        elif re.fullmatch(r"M\d+-\d+\.\d+", ident) and len(cells) == 6:
            yield "task", ident, cells[2], cells[3], cells[4], cells[5]
        elif re.fullmatch(r"M\d+-\d+", ident) and len(cells) == 6:
            yield "task", ident, cells[1], cells[3], cells[4], cells[5]
        elif re.fullmatch(r"H-\d{3}", ident) and len(cells) == 5:
            yield "task", ident, cells[1], cells[2], cells[3], cells[4]


def check_ledger(ledger):
    for kind, ident, state, date, approval, evidence in ledger_rows(ledger):
        where = f"STATUS.md {ident}"
        if kind == "gate":
            if state not in GATE_STATES:
                errors.append(f"{where}: invalid gate state {state!r}")
            elif state != "PENDING":
                if not DATE.match(date):
                    errors.append(f"{where}: gate decision needs a YYYY-MM-DD date")
                if not PERMALINK.search(evidence):
                    errors.append(f"{where}: gate decision needs the owner comment permalink (D-025)")
            continue
        if state not in TASK_STATES:
            errors.append(f"{where}: invalid task state {state!r}")
            continue
        if state in NEEDS_APPROVAL:
            if not DATE.match(date):
                errors.append(f"{where}: state {state} needs a YYYY-MM-DD date")
            if ident not in LEGACY_APPROVAL and not PERMALINK.search(approval):
                errors.append(f"{where}: state {state} needs the owner approval comment permalink (D-025)")
        if state in {"ACCEPTED", "REJECTED / REWORK"} and ident not in LEGACY_DECISION:
            if not PERMALINK.search(evidence):
                errors.append(f"{where}: state {state} needs the owner decision comment permalink (D-025)")


def slug(heading):
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", heading)
    text = text.replace("`", "").replace("*", "").strip().lower()
    text = re.sub(r"[^\w\- ]", "", text)
    return text.replace(" ", "-")


def anchors_of(path):
    seen, out = {}, set()
    in_fence = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        m = HEADING.match(line)
        if m:
            base = slug(m.group(2))
            n = seen.get(base, 0)
            seen[base] = n + 1
            out.add(base if n == 0 else f"{base}-{n}")
    return out


for rel in REQUIRED:
    if not (ROOT / rel).is_file():
        errors.append(f"Required file missing: {rel}")

if not errors:
    catalog = (ROOT / "docs/TASK_CATALOG.md").read_text(encoding="utf-8")
    roadmap = (ROOT / "docs/ROADMAP.md").read_text(encoding="utf-8")
    catalog_ids = CATALOG_ID.findall(catalog)
    roadmap_ids = ROADMAP_ID.findall(roadmap)
    if len(catalog_ids) != 43 or len(catalog_ids) != len(set(catalog_ids)):
        errors.append(f"Expected exactly 43 unique base tasks; found {len(catalog_ids)}.")
    if catalog_ids != roadmap_ids:
        errors.append("Task catalog IDs do not match roadmap order.")
    ledger = (ROOT / "docs/STATUS.md").read_text(encoding="utf-8")
    ledger_ids = [row[1] for row in ledger_rows(ledger) if row[0] == "task"]
    if len(ledger_ids) != len(set(ledger_ids)):
        errors.append("Duplicate task ID in ledger.")
    if not set(catalog_ids).issubset(set(ledger_ids)):
        errors.append("STATUS.md must list all 43 base task IDs.")
    for ident in ledger_ids + SUBTASK_ID.findall(catalog):
        if not VALID_WORK_ID.fullmatch(ident):
            errors.append(f"Invalid work ID: {ident}")
    gates = [row[1] for row in ledger_rows(ledger) if row[0] == "gate"]
    for gate in ("G0", "G1", "G2", "G3", "G4"):
        if gate not in gates:
            errors.append(f"Ledger gate row missing or malformed: {gate}")
    check_ledger(ledger)
    security_model = (ROOT / "docs/SECURITY_MODEL.md").read_text(encoding="utf-8")
    traceability = (ROOT / "docs/TRACEABILITY.md").read_text(encoding="utf-8")
    for i in range(1, 14):
        label = f"S-{i:02d}"
        if label not in security_model or label not in traceability:
            errors.append(f"Security requirement not traced: {label}")
    agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
    for gate in ("G0", "G1", "G2", "G3", "G4"):
        if gate not in roadmap or gate not in agents:
            errors.append(f"Gate is missing from roadmap or agent rules: {gate}")

markdown_paths = sorted(p for p in ROOT.rglob("*.md") if ".git" not in p.relative_to(ROOT).parts)
anchor_cache = {}

for path in markdown_paths:
    rel_path = path.relative_to(ROOT)
    text = path.read_text(encoding="utf-8")
    if not text.endswith("\n"):
        errors.append(f"{rel_path}: missing final newline")
    in_fence = False
    for line in text.splitlines():
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for match in LINK.finditer(INLINE_CODE.sub("", line)):
            raw = match.group(1).strip()
            if raw.startswith("<") and raw.endswith(">"):
                raw = raw[1:-1]
            if raw.startswith(("http://", "https://", "mailto:")):
                continue
            target_part, _, fragment = raw.partition("#")
            target_part = unquote(target_part.split("?", 1)[0])
            target = (path.parent / target_part).resolve() if target_part else path
            if not target.is_relative_to(ROOT):
                errors.append(f"{rel_path}: link escapes repository: {raw}")
                continue
            if not target.exists():
                errors.append(f"{rel_path}: broken relative link: {raw}")
                continue
            if fragment and target.suffix == ".md":
                if target not in anchor_cache:
                    anchor_cache[target] = anchors_of(target)
                if unquote(fragment).lower() not in anchor_cache[target]:
                    errors.append(f"{rel_path}: broken anchor: {raw}")

if errors:
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    sys.exit(1)

print(f"PASS: {len(markdown_paths)} Markdown files inspected (links, images, anchors), "
      f"{len(catalog_ids)} ordered tasks, 13 security requirements, G0-G4 present, "
      "ledger decision links well-formed.")
print("Note: This checks repository documentation only, not code or cryptography.")
