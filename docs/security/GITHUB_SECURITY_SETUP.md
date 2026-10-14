# Septuvo GitHub repository protection

**Repository:** `neosmith20/Septuvo` — **public** personal-account repository since 2026-10-14 (owner decision [D-022](../DECISIONS.md)).
**Last verified:** 2026-10-14 (UTC), read back through the GitHub API. Re-verify after any settings change; this page records state, it does not enforce it.

## Active controls on `main`

Ruleset **`Septuvo — protect main`** (source: [MAIN_RULESET.json](MAIN_RULESET.json)) — enforcement **Active**, target `refs/heads/main`, **bypass list empty** (nobody, including the owner, can bypass it).

| Rule | Effect |
| --- | --- |
| Require a pull request before merging | No direct pushes to `main`. 0 required approvals (a solo owner cannot approve their own PR); conversation resolution required; stale reviews dismissed on push |
| Required status check `repository-integrity` | The [Repository Integrity](../../.github/workflows/repository-integrity.yml) workflow must pass, and the branch must be up to date with `main` (strict) |
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
| GitHub Actions permissions / workflow token default | **Not verified** | The agent connection cannot read Actions settings. The integrity workflow itself requests only `contents: read` and pins `actions/checkout` to a full commit SHA |
| CodeQL code scanning (default setup) | **Enabled** | Languages: GitHub Actions, Python, Rust; default query suite; runs on PRs, pushes and weekly. First run on PR #7 passed |

Alert lists (Security tab) are not readable through the agent connection; the owner reviews them in the GitHub UI.

## Who can change what

- **Agents** (Claude, ChatGPT/Codex or others) work through the owner's GitHub account under [D-023](../DECISIONS.md). They may push task branches and open pull requests. They do **not** merge, push to `main`, edit [STATUS.md](../STATUS.md), or change repository settings, rulesets or visibility. The ruleset makes direct pushes to `main` impossible for everyone; the remaining limits are procedural because GitHub cannot tell agent actions from owner actions on one account.
- **Owner** reviews each PR's files and check result, then merges. Owner alone records approvals in STATUS.md (through a PR, as with PR #6).
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
