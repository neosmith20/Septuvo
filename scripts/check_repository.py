#!/usr/bin/env python3
"""Standard-library-only documentation integrity check. Not a security audit."""

from pathlib import Path
from urllib.parse import unquote
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = (
    "README.md",
    "AGENTS.md",
    "LICENSE",
    "docs/ROADMAP.md",
    "docs/TASK_CATALOG.md",
    "docs/AGENT_OPERATIONS.md",
    "docs/ARCHITECTURE.md",
    "docs/SECURITY_MODEL.md",
    "docs/TRACEABILITY.md",
    "docs/WORK_ORDER_TEMPLATE.md",
    "docs/TEST_MATRIX.md",
    "docs/DECISIONS.md",
    "docs/PROTOCOL_REVIEW_CHECKLIST.md",
    ".github/dependabot.yml",
    ".github/workflows/repository-integrity.yml",
    "SECURITY.md",
)
LINK = re.compile(r"(?<!!)\[[^\]\n]+\]\(([^)\n]+)\)")
CATALOG_ID = re.compile(r"^### (M\d+-\d+) —", re.MULTILINE)
ROADMAP_ID = re.compile(r"\*\*(M\d+-\d+):\*\*")
errors = []

for rel in REQUIRED:
    if not (ROOT / rel).is_file():
        errors.append(f"Required file missing: {rel}")

if not errors:
    catalog = (ROOT / "docs/TASK_CATALOG.md").read_text(encoding="utf-8")
    roadmap = (ROOT / "docs/ROADMAP.md").read_text(encoding="utf-8")
    catalog_ids = CATALOG_ID.findall(catalog)
    roadmap_ids = ROADMAP_ID.findall(roadmap)
    if not catalog_ids or len(catalog_ids) != len(set(catalog_ids)):
        errors.append("Task catalog IDs are missing or duplicated.")
    if catalog_ids != roadmap_ids:
        errors.append("Task catalog IDs do not match roadmap order.")
    security_model = (ROOT / "docs/SECURITY_MODEL.md").read_text(encoding="utf-8")
    traceability = (ROOT / "docs/TRACEABILITY.md").read_text(encoding="utf-8")
    for i in range(1, 14):
        label = f"S-{i:02d}"
        if label not in security_model or label not in traceability:
            errors.append(f"Security requirement not traced: {label}")
    for gate in ("G0", "G1", "G2", "G3", "G4"):
        if gate not in roadmap or gate not in (ROOT / "AGENTS.md").read_text(encoding="utf-8"):
            errors.append(f"Gate is missing from roadmap or agent rules: {gate}")

markdown_paths = [ROOT / "README.md", ROOT / "AGENTS.md", ROOT / "SECURITY.md"]
markdown_paths += sorted((ROOT / "docs").rglob("*.md"))
markdown_paths += sorted((ROOT / ".github").rglob("*.md"))

for path in markdown_paths:
    if not path.is_file():
        continue
    text = path.read_text(encoding="utf-8")
    if not text.endswith("\n"):
        errors.append(f"{path.relative_to(ROOT)}: missing final newline")
    for match in LINK.finditer(text):
        raw = match.group(1).strip()
        if raw.startswith("<") and raw.endswith(">"):
            raw = raw[1:-1]
        if raw.startswith(("#", "http://", "https://", "mailto:")):
            continue
        target_path = unquote(raw.split("#", 1)[0].split("?", 1)[0])
        if not target_path:
            continue
        target = (path.parent / target_path).resolve()
        if not target.is_relative_to(ROOT):
            errors.append(f"{path.relative_to(ROOT)}: link escapes repository: {raw}")
        elif not target.exists():
            errors.append(f"{path.relative_to(ROOT)}: broken relative link: {raw}")

if errors:
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    sys.exit(1)

print(f"PASS: {len(markdown_paths)} Markdown files inspected, "
      f"{len(catalog_ids)} ordered tasks, 13 security requirements, G0-G4 present.")
print("Note: This checks repository documentation only, not code or cryptography.")
