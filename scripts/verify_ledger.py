#!/usr/bin/env python3
"""Verify that every decision mirrored in docs/STATUS.md matches an owner comment (D-025).

For each ledger row that records an owner decision, fetch the linked GitHub
comment and require that it was written by the repository owner, sits on the
linked issue or pull request, and reads exactly the expected decision text
(for example `APPROVED WORK ORDER M0-02`, or the milestone form
`APPROVED WORK ORDER M0` allowed by D-030), on the UTC date recorded in the row.

Standard library only. Needs network access to api.github.com and uses
GITHUB_TOKEN when set. Exit codes: 0 verified, 1 mismatch found,
2 verification impossible (network/API error) - the check fails closed.
This catches mistakes and unsupported ledger rows; it cannot prove who typed a
comment, because agents work through the owner's account (D-023).
"""

import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import urlparse, urlunparse

ROOT = Path(__file__).resolve().parents[1]
REPO = os.environ.get("GITHUB_REPOSITORY", "neosmith20/Septuvo")
API = "https://api.github.com"
PERMALINK = re.compile(
    r"https://github\.com/(?P<repo>[^/\s()]+/[^/\s()]+)/(?:issues|pull)/(?P<num>\d+)#issuecomment-(?P<cid>\d+)"
)
NEEDS_APPROVAL = {"READY", "IN PROGRESS", "IN REVIEW", "ACCEPTED", "REJECTED / REWORK"}
# Rows recorded by the owner directly before D-025 (2026-10-14).
LEGACY_APPROVAL = {"H-001", "H-002", "H-003", "H-004"}
LEGACY_DECISION = LEGACY_APPROVAL | {"M0-01", "M0-01.1"}


def milestone_of(ident):
    """M1-02 -> M1, M0-01.1 -> M0, N1-03 -> N1; H tasks have no milestone (D-030)."""
    m = re.match(r"^((?:M|N)\d+)-\d+", ident)
    return m.group(1) if m else None


def decision_texts(word, ident):
    texts = [f"{word} {ident}"]
    if milestone_of(ident):
        texts.append(f"{word} {milestone_of(ident)}")
    return texts


class Unverifiable(Exception):
    pass


def build_validated_url(base_url: str, path: str) -> str:
    try:
        if "/../" in path or re.search(r"/%2e%2e/", path, re.IGNORECASE):
            raise ValueError("Invalid path")
        parsed = urlparse(base_url)
        parsed = parsed._replace(path=path)
        return urlunparse(parsed)
    except Exception:
        raise ValueError("Invalid URL")


def api_get(path):
    url = build_validated_url(API, path)
    request = urllib.request.Request(url, headers={
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "septuvo-verify-ledger",
    })
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            return json.load(response)
    except urllib.error.HTTPError as exc:
        if exc.code == 404:
            return None
        raise Unverifiable(f"GitHub API {path}: HTTP {exc.code}") from exc
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        raise Unverifiable(f"GitHub API {path}: {exc}") from exc


def ledger_rows(text):
    for line in text.splitlines():
        if not line.startswith("| "):
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


def main():
    ledger = (ROOT / "docs/STATUS.md").read_text(encoding="utf-8")
    repo_info = api_get(f"/repos/{REPO}")
    if not repo_info:
        raise Unverifiable(f"repository {REPO} not found")
    owner = repo_info["owner"]["login"]
    errors, verified = [], 0

    def verify(ident, cell, expected_texts, date, purpose):
        nonlocal verified
        links = PERMALINK.findall(cell)
        if not links:
            errors.append(f"{ident}: no comment permalink for {purpose}")
            return
        expected_repo = REPO.lower()
        for repo, num, cid in links:
            if repo.lower() != expected_repo:
                continue
            comment = api_get(f"/repos/{REPO}/issues/comments/{cid}")
            if comment is None:
                continue
            ok_author = comment["user"]["login"] == owner and comment.get("author_association") == "OWNER"
            ok_place = comment["issue_url"].rstrip("/").endswith(f"/issues/{num}")
            ok_text = comment["body"].strip() in expected_texts
            ok_date = date is None or comment["created_at"][:10] == date
            if ok_author and ok_place and ok_text and ok_date:
                verified += 1
                return
            problems = [name for name, ok in (("not written by the owner", ok_author),
                                              (f"not on #{num}", ok_place),
                                              (f"text is not one of {expected_texts!r}", ok_text),
                                              (f"date differs from {date}", ok_date)) if not ok]
            errors.append(f"{ident}: {purpose} comment {cid}: " + "; ".join(problems))
            return
        errors.append(f"{ident}: {purpose}: linked comment not found in {REPO}")

    for kind, ident, state, date, approval, evidence in ledger_rows(ledger):
        if kind == "gate":
            if state in {"PASSED", "FAILED"}:
                verify(ident, evidence, [f"GATE {ident} {state}"], date, "gate decision")
            continue
        if state not in NEEDS_APPROVAL:
            continue
        decided = state in {"ACCEPTED", "REJECTED / REWORK"}
        if ident not in LEGACY_APPROVAL:
            verify(ident, approval, decision_texts("APPROVED WORK ORDER", ident), None if decided else date, "approval")
        if decided and ident not in LEGACY_DECISION:
            word = "ACCEPTED" if state == "ACCEPTED" else "REJECTED"
            verify(ident, evidence, decision_texts(word, ident), date, "acceptance" if word == "ACCEPTED" else "rejection")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"PASS: {verified} owner decision comment(s) verified for the STATUS.md mirror ({REPO}, owner {owner}).")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Unverifiable as exc:
        print(f"ERROR: could not verify the ledger: {exc}", file=sys.stderr)
        sys.exit(2)
