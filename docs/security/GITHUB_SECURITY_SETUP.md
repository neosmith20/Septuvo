# Septuvo GitHub repository protection

**Repository:** `neosmith20/Septuvo` — **public** personal-account repository since 2026-10-14 (owner decision [D-022](../DECISIONS.md)).
**Last verified:** 2026-10-14 (UTC), read back through the GitHub API. Re-verify after any settings change; this page records state, it does not enforce it.

## Active controls on `main`

Ruleset **`Septuvo — protect main`** (source: [MAIN_RULESET.json](MAIN_RULESET.json)) — enforcement **Active**, target `refs/heads/main`, **bypass list empty** (nobody, including the owner, can bypass it).

| Rule | Effect |
| --- | --- |
| Require a pull request before merging | No direct pushes to `main`. 0 required approvals (a solo owner cannot approve their own PR); conversation resolution required; stale reviews dismissed on push |
| Required status check `repository-integrity` | The [Repository Integrity](../../.github/workflows/repository-integrity.yml) workflow must pass, and the branch must be up to date with `main` (strict). The check scripts always run from the base branch against the pull request's files, so a pull request cannot weaken the checks that judge it (H-008). Binding the check to the GitHub Actions app (integration ID 15368) is recorded in [MAIN_RULESET.json](MAIN_RULESET.json), but the live ruleset read back on 2026-10-14 was **not** bound yet; the owner applies it under the ruleset's required status checks. It checks documentation integrity and verifies that every decision mirrored in STATUS.md matches an owner comment ([`verify_ledger.py`](../../scripts/verify_ledger.py)) |
| Restrict deletions | `main` cannot be deleted |
| Block force pushes | History on `main` cannot be rewritten |
| Require linear history | Merge with **Squash and merge** (or rebase); merge commits are rejected |

Confirmed in practice: owner ledger change [PR #6](https://github.com/neosmith20/Septuvo/pull/6) went through a pull request, passed `repository-integrity` and was squash-merged.

The ruleset also carries GitHub's default `require_extra_approval_for_unattributed_changes: true`. Its effect with 0 required approvals has not been observed yet; if a PR unexpectedly asks for an approval, review this setting first.

## Security features

| Feature | State (verified 2026-10-14) | Notes |
| --- | --- | --- |
| Secret scanning | **Enabled** | Scans history and new pushes for known credential formats |
| Push protection | **Enabled** | Blocks pushes that contain detected secrets |
| Dependabot security updates | **Enabled** | Opens PRs for vulnerable dependencies (requires Dependabot alerts, so alerts are on) |
| Private vulnerability reporting | **Enabled** | Reporting path for [SECURITY.md](../../SECURITY.md): Security tab → *Report a vulnerability* |
| Dependabot version updates | Configured by [`.github/dependabot.yml`](../../.github/dependabot.yml) | GitHub Actions now; Cargo once a `Cargo.toml` exists (M3) |
| Non-provider secret patterns, validity checks | Disabled | Optional; not required at this stage |
| GitHub Actions workflow token default | **Read-only** — owner-reported 2026-10-14 ("Read repository contents and packages permissions") | The agent connection cannot read Actions settings, so this is the owner's report, not an API read-back. Workflows still declare their own minimum permissions |
| OpenSSF Scorecard workflow | **Enabled** (owner-added, [PR #23](https://github.com/neosmith20/Septuvo/pull/23)) | [`scorecard.yml`](../../.github/workflows/scorecard.yml): runs on pushes to `main`, ruleset changes and weekly; publishes results to the public OpenSSF service and the Code scanning tab. Every action pinned to a full commit SHA (H-007) |
| Aikido Security (third-party GitHub App) | **Installed by the owner** (first pull requests 2026-10-14) | Scans the repository and opens autofix pull requests from `aikido-autofix[bot]` ([PR #26](https://github.com/neosmith20/Septuvo/pull/26), [PR #27](https://github.com/neosmith20/Septuvo/pull/27)). Its pull requests pass through the same ruleset and checks. PRs #31–#35 (2026-10-14) changed the check scripts and workflow; one of them (#32) made the integrity check validate `main` instead of the pull request, which H-008 corrected. An agent review before merging bot pull requests that touch `.github/` or `scripts/` is recommended |
| CodeQL code scanning (default setup) | **Enabled** | Languages: GitHub Actions, Python, Rust; default query suite; runs on PRs, pushes and weekly. First run on PR #7 passed |

Alert lists (Security tab) are not readable through the agent connection; the owner reviews them in the GitHub UI.

## Who can change what

- **Agents** (Claude, ChatGPT/Codex or others) work through the owner's GitHub account under [D-023](../DECISIONS.md). They may push task branches and open pull requests. They do **not** merge, push to `main`, or change repository settings, rulesets or visibility, and they update [STATUS.md](../STATUS.md) only from GitHub evidence, using `scripts/sync_status.py` (D-031). The ruleset makes direct pushes to `main` impossible for everyone; the remaining limits are procedural because GitHub cannot tell agent actions from owner actions on one account.
- **Owner** reviews each PR's files and check results, then merges; **the merge accepts the work** (D-031). The owner authorizes milestones in ordinary words and decides gates with the exact comment `GATE <Gn> PASSED/FAILED`. No agent or bot merges a pull request.
- Commits made by Claude carry `Claude` as the git author, so they can be told apart in history.

## Public repository rules

- Never commit secrets, private keys, tokens, real network details or user data. Push protection is a backstop, not a substitute.
- The Septuvo name and brand kit are now publicly visible, while trademark and domain clearance is still open ([D-016](../DECISIONS.md)). Do not add ™/® symbols or claim rights until cleared.
- Public visibility does not change any engineering gate: the repository still contains no VPN implementation, no audited protocol and no supported release.

## If something goes wrong

- **Locked out of merging** (for example the check workflow breaks): the owner can set the ruleset to *Disabled* under **Settings → Rules → Rulesets**, fix the problem through a PR, and set it back to *Active*. Record what happened.
- **A secret is committed or reported:** rotate/revoke the secret first, then remove it; treat it as exposed even if removed from history.

## Optional tidy-ups and future hardening

- **Settings → General → Pull Requests:** untick *Allow merge commits* so only Squash/Rebase are offered (merge commits are already rejected by the ruleset).
- Add Rust fmt/clippy/test/audit/dependency/license checks during M3, then make the observed check names required.
- Require 1 approving review only once a second trusted reviewer with write access exists; consider a separate agent account at that point or before G3.
- Require signed commits only after every agent/CI identity has a verified signing workflow.
- Protect release tags and publish signed artifacts only after the release process and gate G3 approval.
- Keep 2FA/passkeys on the owner account; periodically review installed GitHub Apps, tokens, deploy keys and sessions.

**Do not** treat these GitHub controls as passing any product gate (G0–G4) in [ROADMAP.md](../ROADMAP.md). The gates concern the VPN product, not repository settings.
