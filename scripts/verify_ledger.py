#!/usr/bin/env python3
"""Verify every decision recorded in docs/STATUS.md against GitHub (D-031).

What each kind of row needs (rules in ledger_rules.py):

- Routine task (H-xxx) marked ACCEPTED: a linked pull request that the owner
  merged into main and that delivers the task (task ID in a commit-subject or
  title scope), merged on the row's date. No approval comment.
- Milestone task (Mx-yy, Nx-yy): an owner authorization - an owner comment in
  ordinary language that names the task or its milestone (or sits on the pull
  request that adds its work order), or a merged pull request that adds
  docs/work-orders/<task or milestone>.md. ACCEPTED rows also need a delivering
  pull request merged by the owner, as above.
- Gate (G0-G4) PASSED/FAILED: an owner comment reading exactly
  `GATE <Gn> PASSED` or `GATE <Gn> FAILED`, on the row's date.
- Decisions recorded before D-031 with exact comments (`APPROVED WORK ORDER X`,
  `ACCEPTED X`) remain valid evidence.

A ledger that lags behind GitHub is not an error: only what the ledger claims
is verified. sync_status.py reports and fixes lag.

Standard library only. Uses GITHUB_TOKEN when set. Exit codes: 0 verified,
1 mismatch, 2 verification impossible (network/API error) - fails closed.
Agents work through the owner's account (D-023), so this catches mistakes and
unsupported rows; it cannot prove who clicked or typed.
"""

import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import urlencode

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ledger_rules as L  # noqa: E402

# The tree to check. CI runs these scripts from the trusted base branch and
# points SEPTUVO_ROOT at the pull request's files (H-008), so a pull request
# cannot weaken the checks that judge it.
ROOT = Path(os.environ.get("SEPTUVO_ROOT") or Path(__file__).resolve().parents[1]).resolve()
REPO = os.environ.get("GITHUB_REPOSITORY", L.REPO)
API = "https://api.github.com"


class Unverifiable(Exception):
    pass


def api_get(path, query=None):
    """GET an api.github.com path; None on 404; Unverifiable on any other failure."""
    if not path.startswith("/") or ".." in path or "%2e" in path.lower() or "?" in path:
        raise Unverifiable(f"refusing unexpected API path {path!r}")
    url = API + path + ("?" + urlencode(query) if query else "")
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
    except (urllib.error.URLError, TimeoutError, OSError, ValueError) as exc:
        raise Unverifiable(f"GitHub API {path}: {exc}") from exc


class GitHub:
    """Cached read-only access to the few objects the ledger links to."""

    def __init__(self, get=api_get, repo=REPO):
        self.get, self.repo, self._cache = get, repo, {}

    def _cached(self, key, path, query=None):
        if key not in self._cache:
            self._cache[key] = self.get(path, query)
        return self._cache[key]

    def owner(self):
        info = self._cached("repo", f"/repos/{self.repo}")
        if not info:
            raise Unverifiable(f"repository {self.repo} not found")
        return info["owner"]["login"]

    def comment(self, cid):
        return self._cached(("comment", cid), f"/repos/{self.repo}/issues/comments/{cid}")

    def pull(self, num):
        return self._cached(("pull", num), f"/repos/{self.repo}/pulls/{num}")

    def pull_files(self, num):
        return self._cached(("files", num), f"/repos/{self.repo}/pulls/{num}/files", {"per_page": 100}) or []

    def pull_commits(self, num):
        return self._cached(("commits", num), f"/repos/{self.repo}/pulls/{num}/commits", {"per_page": 100}) or []

    def delivered(self, num):
        """Task IDs a pull request delivers: scopes of its title and commit subjects."""
        pr = self.pull(num) or {}
        ids = L.delivered_ids(pr.get("title", ""))
        for c in self.pull_commits(num):
            ids |= L.delivered_ids(c.get("commit", {}).get("message", "").split("\n", 1)[0])
        return ids

    def adds_work_order(self, num, names):
        wanted = {f"docs/work-orders/{n}.md" for n in names}
        return any(f.get("filename") in wanted for f in self.pull_files(num))


def merged_by_owner(gh, pr, owner):
    return bool(pr and pr.get("merged_at") and (pr.get("merged_by") or {}).get("login") == owner
                and (pr.get("base") or {}).get("ref") == "main")


def owner_comment(gh, link, owner):
    """The linked comment if the owner wrote it on the linked issue/PR, else None."""
    c = gh.comment(link.group("cid"))
    if not c:
        return None
    ok = (c.get("user", {}).get("login") == owner and c.get("author_association") == "OWNER"
          and c.get("issue_url", "").rstrip("/").endswith(f"/issues/{link.group('num')}"))
    return c if ok else None


def check_authorization(gh, owner, ident, cell, date):
    """Category B: ordinary-language owner comment or merged work-order pull request."""
    names = [ident] + ([L.milestone_of(ident)] if L.milestone_of(ident) else [])
    for link in L.links(cell, L.COMMENT_LINK):
        c = owner_comment(gh, link, owner)
        if not c:
            return f"{ident}: authorization comment {link.group('cid')} is missing or not the owner's"
        text = c.get("body", "").strip()
        named = any(L.mentions(text, n) for n in names)
        on_work_order = bool(gh.pull(link.group("num"))) and gh.adds_work_order(link.group("num"), names)
        if not text or not (named or on_work_order):
            return f"{ident}: authorization comment {link.group('cid')} names neither {' nor '.join(names)} nor sits on its work-order pull request"
        if date and c["created_at"][:10] != date:
            return f"{ident}: authorization comment date {c['created_at'][:10]} differs from {date}"
        return None
    for link in L.links(cell, L.PR_LINK):
        pr = gh.pull(link.group("num"))
        if not merged_by_owner(gh, pr, owner):
            return f"{ident}: authorization pull request #{link.group('num')} was not merged into main by the owner"
        if not gh.adds_work_order(link.group("num"), names):
            return f"{ident}: pull request #{link.group('num')} adds no work order for {' or '.join(names)}"
        return None
    return f"{ident}: no owner authorization link"


def check_completion(gh, owner, ident, cell, date, accepted):
    """ACCEPTED: owner-merged delivering pull request(s) or a legacy owner comment."""
    names = [ident] + ([L.milestone_of(ident)] if L.milestone_of(ident) else [])
    word = "accept" if accepted else "reject"
    for link in L.links(cell, L.COMMENT_LINK):
        c = owner_comment(gh, link, owner)
        if not c:
            return f"{ident}: decision comment {link.group('cid')} is missing or not the owner's"
        text = c.get("body", "")
        if word not in text.lower() or not any(L.mentions(text, n) for n in names):
            return f"{ident}: decision comment {link.group('cid')} does not {word} {' or '.join(names)}"
        if c["created_at"][:10] != date:
            return f"{ident}: decision comment date {c['created_at'][:10]} differs from {date}"
        return None
    pr_links = L.links(cell, L.PR_LINK)
    if not pr_links:
        return f"{ident}: no merged pull request or owner comment as evidence"
    if not accepted:
        return f"{ident}: a rejection needs an owner comment, not a pull request link"
    dates = []
    for link in pr_links:
        num = link.group("num")
        pr = gh.pull(num)
        if not merged_by_owner(gh, pr, owner):
            return f"{ident}: pull request #{num} was not merged into main by the owner"
        if ident not in gh.delivered(num):
            return f"{ident}: pull request #{num} does not deliver {ident} (no `type({ident}): ...` commit or title)"
        dates.append(pr["merged_at"][:10])
    if max(dates) != date:
        return f"{ident}: row date {date} differs from the last merge date {max(dates)}"
    return None


def verify(ledger_text, gh):
    """Return (errors, verified_count) for a STATUS.md text."""
    owner = gh.owner()
    errors, verified = [], 0
    for kind, ident, state, date, approval, evidence in L.ledger_rows(ledger_text):
        problem = None
        if kind == "gate":
            if state not in {"PASSED", "FAILED"}:
                continue
            links = L.links(evidence, L.COMMENT_LINK)
            c = owner_comment(gh, links[0], owner) if links else None
            if not c or c.get("body", "").strip() != f"GATE {ident} {state}" or c["created_at"][:10] != date:
                problem = f"{ident}: gate {state} needs the owner's exact `GATE {ident} {state}` comment dated {date}"
            if problem:
                errors.append(problem)
            else:
                verified += 1
            continue
        if ident in L.LEGACY_BASELINE or state not in L.NEEDS_AUTHORIZATION:
            continue
        decided = state in L.DECIDED
        if L.category(ident) == "B":
            problem = check_authorization(gh, owner, ident, approval, None if decided else date)
            if problem:
                errors.append(problem)
            else:
                verified += 1
        if decided:
            problem = check_completion(gh, owner, ident, evidence, date, state == "ACCEPTED")
            if problem:
                errors.append(problem)
            else:
                verified += 1
    return errors, verified


def main():
    ledger = (ROOT / "docs/STATUS.md").read_text(encoding="utf-8")
    decisions = (ROOT / "docs/DECISIONS.md").read_text(encoding="utf-8")
    errors = L.offline_errors(ledger, decisions)
    gh = GitHub()
    online, verified = verify(ledger, gh)
    errors += online
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"PASS: {verified} ledger record(s) verified against GitHub ({REPO}, owner {gh.owner()}).")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Unverifiable as exc:
        print(f"ERROR: could not verify the ledger: {exc}", file=sys.stderr)
        sys.exit(2)
