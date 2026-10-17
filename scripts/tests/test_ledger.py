"""Tests for the D-031 approval model (ledger_rules, verify_ledger, sync_status).

Run: python3 -m unittest discover -s scripts/tests -t .
No network: GitHub is replaced by an in-memory fake.
"""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
sys.path.insert(0, str(SCRIPTS))

import ledger_rules as L  # noqa: E402
import sync_status as S  # noqa: E402
import verify_ledger as V  # noqa: E402

OWNER = "owner"
R = "https://github.com/neosmith20/Septuvo"


def comment(cid, num, body, user=OWNER, date="2026-10-14"):
    return {"id": cid, "body": body, "user": {"login": user},
            "author_association": "OWNER" if user == OWNER else "NONE",
            "issue_url": f"https://api.github.com/repos/neosmith20/Septuvo/issues/{num}",
            "created_at": f"{date}T10:00:00Z"}


def pull(num, merged_by=OWNER, date="2026-10-14", title="x", base="main", merged=True):
    return {"number": num, "title": title, "base": {"ref": base},
            "merged_at": f"{date}T11:00:00Z" if merged else None,
            "merged_by": {"login": merged_by} if merged else None}


class FakeAPI:
    """Maps API paths to objects, like api_get."""

    def __init__(self):
        self.data = {"/repos/neosmith20/Septuvo": {"owner": {"login": OWNER}}}
        self.fail = False

    def add_comment(self, c):
        self.data[f"/repos/neosmith20/Septuvo/issues/comments/{c['id']}"] = c

    def add_pull(self, p, commits=(), files=()):
        base = f"/repos/neosmith20/Septuvo/pulls/{p['number']}"
        self.data[base] = p
        self.data[base + "/commits"] = [{"commit": {"message": m}} for m in commits]
        self.data[base + "/files"] = [{"filename": f} for f in files]

    def __call__(self, path, query=None):
        if self.fail:
            raise V.Unverifiable("network down")
        if path == "/repos/neosmith20/Septuvo/pulls":
            return [p for k, p in self.data.items()
                    if k.count("/") == 5 and k.startswith("/repos/neosmith20/Septuvo/pulls/")]
        return self.data.get(path)


GATES_G0 = f"""
| G0 | PASSED | 2026-10-14 | [gate]({R}/pull/25#issuecomment-900) |
| G1 | PENDING | — | — |
| G2 | PENDING | — | — |
| G3 | PENDING | — | — |
| G4 | PENDING | — | — |
"""


def ledger(*rows, gates=GATES_G0):
    return gates + "\n" + "\n".join(rows) + "\n"


def h_row(ident, state="ACCEPTED", date="2026-10-14", approval=L.ROUTINE, evidence=f"[PR #28]({R}/pull/28)"):
    return f"| {ident} | {state} | {date} | {approval} | {evidence} |"


def m_row(ident, state="READY", date="2026-10-14", approval=f"[auth]({R}/pull/24#issuecomment-801)", evidence="—"):
    return f"| {ident} | {state} | — | {date} | {approval} | {evidence} |"


class Base(unittest.TestCase):
    def setUp(self):
        self.api = FakeAPI()
        self.api.add_comment(comment(900, 25, "GATE G0 PASSED"))
        self.api.add_comment(comment(801, 24, "Approved M1, go ahead."))
        self.api.add_pull(pull(24), commits=["chore(H-006): lean process"], files=["docs/work-orders/M1.md"])
        self.api.add_pull(pull(28), commits=["chore(H-007): pin scorecard"])
        self.api.add_pull(pull(30), commits=["docs(M1-01): threats", "docs(M1-02): network"])

    def errors(self, text):
        errs = L.offline_errors(text)
        online, _ = V.verify(text, V.GitHub(get=self.api))
        return errs + online


class Tracks(unittest.TestCase):
    def test_every_track_needs_g0(self):
        for track in ("N1", "N2", "N3"):
            self.assertEqual(L.REQUIRED_GATE[track], "G0")
            self.assertEqual(L.milestone_of(track + "-01"), track)


class CategoryA(Base):
    def test_routine_merge_is_acceptance(self):
        self.assertEqual(self.errors(ledger(h_row("H-007"))), [])

    def test_merge_by_someone_else_is_rejected(self):
        self.api.add_pull(pull(28, merged_by="aikido-autofix[bot]"), commits=["chore(H-007): x"])
        self.assertTrue(any("not merged into main by the owner" in e for e in self.errors(ledger(h_row("H-007")))))

    def test_unmerged_pull_request_is_rejected(self):
        self.api.add_pull(pull(28, merged=False), commits=["chore(H-007): x"])
        self.assertTrue(self.errors(ledger(h_row("H-007"))))

    def test_pull_request_must_deliver_the_task(self):
        # Mentioning H-009 in a body or as "next" does not count as delivering it.
        self.assertTrue(any("does not deliver H-009" in e for e in self.errors(ledger(h_row("H-009")))))

    def test_merge_date_must_match(self):
        self.assertTrue(any("differs" in e for e in self.errors(ledger(h_row("H-007", date="2026-10-15")))))

    def test_merge_into_other_branch_is_rejected(self):
        self.api.add_pull(pull(28, base="dev"), commits=["chore(H-007): x"])
        self.assertTrue(self.errors(ledger(h_row("H-007"))))


class CategoryB(Base):
    def test_milestone_authorized_in_ordinary_language(self):
        self.assertEqual(self.errors(ledger(m_row("M1-02"))), [])

    def test_milestone_acceptance_by_merge(self):
        row = m_row("M1-01", state="ACCEPTED", evidence=f"[PR #30]({R}/pull/30)")
        self.assertEqual(self.errors(ledger(row)), [])

    def test_missing_authorization_for_new_scope(self):
        row = m_row("M1-01", state="ACCEPTED", approval="—", evidence=f"[PR #30]({R}/pull/30)")
        self.assertTrue(any("authorization" in e for e in self.errors(ledger(row))))

    def test_authorization_by_non_owner_is_rejected(self):
        self.api.add_comment(comment(802, 24, "Approved M1", user="someone"))
        row = m_row("M1-02", approval=f"[x]({R}/pull/24#issuecomment-802)")
        self.assertTrue(any("not the owner's" in e for e in self.errors(ledger(row))))

    def test_unrelated_owner_comment_is_not_authorization(self):
        self.api.add_comment(comment(803, 28, "Looks good"))
        row = m_row("M1-02", approval=f"[x]({R}/pull/28#issuecomment-803)")
        self.assertTrue(any("names neither" in e for e in self.errors(ledger(row))))

    def test_short_comment_on_work_order_pull_request_authorizes(self):
        self.api.add_comment(comment(804, 24, "Approved"))
        row = m_row("M1-03", approval=f"[x]({R}/pull/24#issuecomment-804)")
        self.assertEqual(self.errors(ledger(row)), [])

    def test_merged_work_order_pull_request_authorizes(self):
        row = m_row("M1-03", approval=f"[work order]({R}/pull/24)")
        self.assertEqual(self.errors(ledger(row)), [])

    def test_pull_request_without_work_order_does_not_authorize(self):
        row = m_row("M1-03", approval=f"[x]({R}/pull/28)")
        self.assertTrue(any("adds no work order" in e for e in self.errors(ledger(row))))

    def test_legacy_exact_comments_still_valid(self):
        self.api.add_comment(comment(805, 21, "ACCEPTED M0-02"))
        self.api.add_comment(comment(806, 18, "APPROVED WORK ORDER M0-02"))
        row = m_row("M0-02", state="ACCEPTED", approval=f"[a]({R}/issues/18#issuecomment-806)",
                    evidence=f"[b]({R}/pull/21#issuecomment-805)")
        self.assertEqual(self.errors(ledger(row)), [])

    def test_approval_comment_is_not_acceptance(self):
        row = m_row("M1-02", state="ACCEPTED", evidence=f"[x]({R}/pull/24#issuecomment-801)")
        self.assertTrue(any("does not accept" in e for e in self.errors(ledger(row))))


class CategoryC(Base):
    def test_gate_needs_exact_owner_comment(self):
        self.api.add_comment(comment(900, 25, "looks good, pass it"))
        self.assertTrue(any("exact" in e for e in self.errors(ledger())))

    def test_gate_by_non_owner_is_rejected(self):
        self.api.add_comment(comment(900, 25, "GATE G0 PASSED", user="someone"))
        self.assertTrue(self.errors(ledger()))

    def test_crypto_milestone_cannot_start_before_g1(self):
        row = m_row("M3-01", approval=f"[x]({R}/pull/24#issuecomment-801)")
        self.assertTrue(any("requires gate G1 PASSED" in e for e in L.offline_errors(ledger(row))))

    def test_m1_cannot_start_before_g0(self):
        gates = GATES_G0.replace("| G0 | PASSED | 2026-10-14 | [gate]({R}/pull/25#issuecomment-900) |".format(R=R),
                                 "| G0 | PENDING | — | — |")
        self.assertTrue(any("requires gate G0" in e for e in L.offline_errors(ledger(m_row("M1-02"), gates=gates))))

    def test_candidate_crypto_cannot_be_marked_decided_before_g1(self):
        decisions = "| D-011 | Key establishment | X25519 + ML-KEM-768 hybrid | **Owner decided 2026-10-14** | — |\n"
        self.assertTrue(L.offline_errors(ledger(), decisions))
        ok = "| D-011 | Key establishment | X25519 + ML-KEM-768 hybrid | Candidate — **not approved handshake** | — |\n"
        self.assertEqual(L.offline_errors(ledger(), ok), [])

    def test_fenced_example_rows_are_ignored(self):
        text = ledger("```", m_row("M3-01", state="ACCEPTED"), "```")
        self.assertEqual(L.offline_errors(text), [])

    def test_network_failure_fails_closed(self):
        self.api.fail = True
        with self.assertRaises(V.Unverifiable):
            V.verify(ledger(h_row("H-007")), V.GitHub(get=self.api))

    def test_legacy_rows_cannot_be_altered(self):
        row = "| H-001 | ACCEPTED | 2026-10-14 | Something else | Documentation workflow fixes: [PR #4] |"
        self.assertTrue(any("legacy" in e for e in L.offline_errors(ledger(row))))


class StaleLedgerAndSync(Base):
    def test_stale_ledger_does_not_block(self):
        # M1-01 merged in PR #30 but still READY: verification passes; sync reports it.
        self.assertEqual(self.errors(ledger(m_row("M1-01"))), [])

    def test_sync_marks_authorized_work_accepted_and_result_verifies(self):
        text = ledger(m_row("M1-01"), m_row("M1-02"), h_row("H-006", evidence=f"[PR #24]({R}/pull/24)"))
        found = S.deliveries(V.GitHub(get=self.api), OWNER)
        updates, new_h, _ = S.plan(text, found)
        self.assertEqual(set(updates), {"M1-01", "M1-02"})
        self.assertEqual([h[0] for h in new_h], ["H-007"])
        written = S.apply(text, updates, new_h)
        self.assertEqual(self.errors(written), [])

    def test_sync_never_accepts_unauthorized_milestone_work(self):
        text = ledger(m_row("M1-01", state="NOT STARTED", date="—", approval="—"))
        updates, _, notes = S.plan(text, {"M1-01": [("2026-10-14", 30)]})
        self.assertEqual(updates, {})
        self.assertTrue(any("owner decision needed" in n for n in notes))

    def test_sync_ignores_mentions_outside_scopes(self):
        self.api.add_pull(pull(40), commits=["feat(N1-02): policy", "docs: next is N1-03"])
        found = S.deliveries(V.GitHub(get=self.api), OWNER)
        self.assertIn("N1-02", found)
        self.assertNotIn("N1-03", found)

    def test_sync_ignores_pull_requests_merged_by_others(self):
        self.api.add_pull(pull(41, merged_by="aikido-autofix[bot]"), commits=["fix(H-009): x"])
        self.assertNotIn("H-009", S.deliveries(V.GitHub(get=self.api), OWNER))


class DeliveredIds(unittest.TestCase):
    def test_scopes(self):
        self.assertEqual(L.delivered_ids("feat(N1-01): x"), {"N1-01"})
        self.assertEqual(L.delivered_ids("docs(M1-01, M1-02): y"), {"M1-01", "M1-02"})
        self.assertEqual(L.delivered_ids("N1-03, N1-04: z"), {"N1-03", "N1-04"})
        self.assertEqual(L.delivered_ids("fix(security): b"), set())
        self.assertEqual(L.delivered_ids("Merge branch 'main'"), set())

    def test_mentions_is_token_exact(self):
        self.assertTrue(L.mentions("Approved M1, go ahead", "M1"))
        self.assertFalse(L.mentions("docs(M1-01)", "M1"))
        self.assertFalse(L.mentions("M0-01.1", "M0-01"))


class OwnerQuestionsCheck(unittest.TestCase):
    """check_repository.py rejects a spec that cites an undefined owner question."""

    def test_undefined_question_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "repo"
            shutil.copytree(REPO_ROOT, root, ignore=shutil.ignore_patterns(".git", "target"))
            spec = root / "docs/spec/threats.md"
            spec.write_text(spec.read_text(encoding="utf-8") + "\nSee Q99.\n", encoding="utf-8")
            env = dict(os.environ, SEPTUVO_ROOT=str(root))
            run = subprocess.run([sys.executable, str(SCRIPTS / "check_repository.py")],
                                 env=env, capture_output=True, text=True)
            self.assertNotEqual(run.returncode, 0)
            self.assertIn("Q99", run.stdout + run.stderr)


if __name__ == "__main__":
    unittest.main()
