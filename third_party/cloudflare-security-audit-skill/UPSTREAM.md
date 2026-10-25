# Cloudflare `security-audit` skill: pinned upstream copy

This folder is an **unmodified copy** of Cloudflare's coding-agent security-audit skill, pinned to one reviewed revision for Septuvo's AI-assisted adversarial audits (H-023, D-044). How and when it is used: [docs/security/AI_ASSISTED_AUDIT.md](../../docs/security/AI_ASSISTED_AUDIT.md).

| Item | Value |
| --- | --- |
| Upstream | https://github.com/cloudflare/security-audit-skill |
| Commit | `c1c8a8c1471069fb0e188eeaff69b8e8db6564a8` (branch `main`, committed 2026-09-14, "Clarify guidance and full audit modes") |
| Git tree of the commit | `8cd03bb3941496e441d6248aaae2429a35b93cc3` |
| Git tree of `skills/security-audit/` | `ccbc33ed119b28c3ef7b44a7e2081ab191ad63af` |
| Files | All 22 files of that commit: `LICENSE`, `README.md` and the 20 files of `skills/security-audit/`. Nothing added, removed or changed |
| Checksums | [`SHA256SUMS`](SHA256SUMS), one SHA-256 per file |
| Licence | MIT, Copyright (c) 2025-2026 Cloudflare, Inc. ([`LICENSE`](LICENSE)). Development tooling only: never compiled into, linked with or shipped in any Septuvo product |
| Copied | 2026-10-24 (UTC) from a fresh clone, compared byte for byte (`diff -r`) |

## What it contains

- **Instructions** (Markdown): the six-phase audit method, attack classes and domain companions. They are prompts for a coding agent, not code.
- **Two validators** (`validate-findings.cjs`, `validate-coverage-ledger.cjs`): zero-dependency Node.js scripts that **read** `findings.json` and `coverage-ledger.json` and report schema errors. Reviewed: they use only Node's built-in `fs`, `path` and `util` modules, read the files named on the command line, and write nothing and contact nothing. Their tests (`*.test.cjs`) write and remove fixtures in the system temporary folder.
- `report-schema.json`: the findings format.

The validators are third-party tooling run by the auditing agent **outside** the target sandbox, on the run's own output files. They are not Septuvo-authored code (D-005) and never process Septuvo's source or run it.

## Checking the copy

From the repository root:

```sh
cd third_party/cloudflare-security-audit-skill && sha256sum -c SHA256SUMS
```

Every line must say `OK`. To check against upstream as well:

```sh
git clone https://github.com/cloudflare/security-audit-skill /some/empty/folder
git -C /some/empty/folder rev-parse c1c8a8c1471069fb0e188eeaff69b8e8db6564a8:skills/security-audit   # ccbc33ed…
diff -r /some/empty/folder/skills third_party/cloudflare-security-audit-skill/skills
```

## Updating the pin

Only through a reviewed pull request:

1. Choose a newer upstream commit at least **two weeks** old (the D-035 rule for dependencies).
2. Read the upstream diff in full: the instructions change how audits behave, and the validators are code.
3. Replace the files, regenerate `SHA256SUMS`, update this table.
4. Note in the pull request what changed in the method, and whether earlier runs stay comparable (the skill treats runs as additive).
5. The owner's merge approves the new pin (D-031).
