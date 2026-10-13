# Septuvo GitHub repository protection — owner setup instructions

**Repository:** `neosmith20/Septuvo` (private personal-account repository)
**Current status:** Settings below are **proposed, not activated**. GitHub's rulesets API returned: `403 — Upgrade to GitHub Pro or make this repository public to enable this feature.` The GitHub connector also cannot modify repository admin/security settings. **Do not change repository visibility to work around this.**

## Activation order (important)

1. Keep the repository **private**. If the account becomes eligible for private repository rulesets (e.g. GitHub Pro), open **Settings → Rules → Rulesets → New ruleset → New branch ruleset**.
2. Merge the reviewed security-workflow PR through the normal existing process before making `repository-integrity` a required status check. Confirm that the `Repository Integrity / repository-integrity` job has **actually completed successfully** on the repository.
3. Create a branch ruleset with **Ruleset name:** `Septuvo — protect main`, **Enforcement:** `Active`, **Target branches:** `main` (exact branch; not `*`), **Bypass list:** empty by default.
4. Select **Restrict deletions**, **Block force pushes**, **Require linear history**, and **Require a pull request before merging**.
5. Configure pull requests for **0 required approving reviews for now** (solo owner cannot approve own PR), **Require conversation resolution: ON**, **Require code owner review: OFF**, **Require approval of most recent push: OFF**. Add a one-review requirement **only after another trusted collaborator with write access can approve**.
6. Enable **Require status checks to pass**, select the **existing** status check `repository-integrity` from the actual successful GitHub Actions job, and enable **Require branch to be up to date** (strict). Do not type a guessed check name or mandate a nonexistent job.
7. Save the ruleset, verify enforcement is **Active**, and confirm `main` rejects a direct push, deletion and force-push (use UI/permissions or an appropriate controlled test; do not attempt destructive tests on the live branch).
8. Ensure repository merge methods are compatible with linear history: **allow Squash merge** (recommended) and optionally Rebase; **disable Merge commits** if desired. Those are separate settings under **Settings → General → Pull Requests**. This is an owner-only UI action; not performed here.

A prospective API body is at [MAIN_RULESET.json](MAIN_RULESET.json). It is a **prepared template, not an applied GitHub rule**. It can be reviewed and submitted with admin-authorized GitHub API access once eligible; do not assume uploading the JSON to the repository activates it. Explicitly confirm ruleset permissions and the latest API schema first.

**Rationale:** Enforced PR flow and CI without accidental solo-maintainer lockout. Do not add required signed commits until each agent/CI identity has a working signing workflow and a verified test merge. Do not require a second reviewer until such a reviewer exists. Keep emergency changes via reviewed PRs; avoid broad “admin bypass” roles by default.

## Security & analysis (manual UI review)

Go to **Settings → Advanced Security** (GitHub may group this under Security and quality), review feature availability for this private *user-owned* repository:

| Feature | Recommendation | Scope / caveat |
| --- | --- | --- |
| Dependabot alerts | **Enable** | Available for private repositories; requires settings toggle; not switched on by adding a YAML file |
| Dependabot security updates | **Enable**, if offered | Separate security-update setting; generates PRs for vulnerable dependencies |
| Dependabot version updates | **Prepared** by `.github/dependabot.yml` | Starts from default branch once merged; covers GitHub Actions now and Cargo once manifests exist |
| GitHub Actions | **Enable** and allow only needed actions | Current workflow needs pinned `actions/checkout`; no broad write permission or secrets |
| Actions workflow token | **Read-only by default** | Settings → Actions → General → Workflow permissions; minimize privileges |
| Secret scanning / push protection | **Enable if eligible** | Free features differ for private *personal* repositories; do not claim active unless shown as active |
| CodeQL/GitHub Code Security | **Only if eligible/licensed** | Private personal-repo access may not be available; do not enable a paid feature or claim code scanning without verification |
| Private vulnerability reporting | Not a substitute for a private reporting channel today | Feature is generally designed for public repositories; maintain [SECURITY.md](../../SECURITY.md) and agree on a secure channel before launch |
| Branch and release permissions | Keep owner/admin access narrow | Review installed GitHub Apps/SSH keys/agents; revoke unused access |
| Account protection | **Enable 2FA/passkey** | Keep recovery codes secure; audit PATs, deploy keys and active sessions |

## Verify settings and state

After enabling any control, return to Settings and confirm its effective state. Check PR workflow results from the **Actions** tab, and test a disposable PR against `main` to confirm required checks and approval settings behave as expected. Document date, operator and screenshots or settings export privately.

**Do not** enable all five security gates in [ROADMAP.md](../ROADMAP.md) simply because GitHub controls are activated. Those gates concern the VPN product, not GitHub settings. There is no implemented cryptographic protocol or released application yet.

## Future hardened settings once the team grows

- Require **1 or 2 independent reviews**, dismiss stale approvals, and restrict bypass to narrowly justified emergency maintainers.
- Enable required signed commits only after automation signing is verified.
- Add Rust fmt/clippy/test/audit/dependency/license checks during M3, **then** mark observed stable check names required.
- Consider secret scanning, CodeQL and artifact attestation according to actual plan/eligibility.
- Protect release tags and publish signed artifacts only after release process and gate G3 approval.
