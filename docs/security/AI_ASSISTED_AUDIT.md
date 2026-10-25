# AI-assisted adversarial security audit (H-023, D-044)

**Status: prepared, not run.** This document is the integration and procedure for running Cloudflare's `security-audit` skill against Septuvo. **No audit has been run.** A run needs the owner's separate, explicit authorization for a named commit (section 6, step 0).

**What this is not:**
- It is **not** an independent security review. It **supplements, and never replaces**, the qualified human reviews that gates **G1** (protocol design) and **G3** (production release) require.
- It is **not** evidence that anything is secure. **One clean run proves nothing**: the skill's authors report that a single run found roughly half of what repeated runs found. Every report states its coverage and its gaps.
- It never passes a gate, accepts work or changes a decision. The owner does those (D-031).

## 1. The tool

Cloudflare's [`security-audit`](https://github.com/cloudflare/security-audit-skill) skill (MIT) turns a coding agent such as Claude Code into a structured auditor. Its full-audit mode runs six phases:

1. **Reconnaissance:** architecture, trust boundaries, entry points, and a deterministic **coverage ledger**.
2. **Coverage-led hunting:** isolated hunter agents, then critic agents that look for gaps.
3. **Candidate validation:** every candidate goes to a fresh agent that tries to disprove it.
4. **Structured output:** `findings.json` with `confirmed`, `needs_validation` and `rejected` records, checked against a schema.
5. **Independent record verification:** fresh agents re-check every final claim against the source.
6. **Report:** `REPORT.md`, `FINDINGS-DETAIL.md` and `NEEDS-VALIDATION.md`, derived from the verified records.

Only `confirmed` records get a severity, and only with a complete source trace and a bounded observed result. A blocked hypothesis stays `needs_validation`, with the exact missing fact. Runs are **additive**: a new run reads earlier ledgers and findings, re-checks changed code and targets earlier gaps.

## 2. The integration

| Choice | What we do | Why |
| --- | --- | --- |
| **Source** | A pinned, unmodified copy in [`third_party/cloudflare-security-audit-skill/`](../../third_party/cloudflare-security-audit-skill/UPSTREAM.md), commit `c1c8a8c1471069fb0e188eeaff69b8e8db6564a8` (2026-09-14), with a SHA-256 for every file | **Reproducible:** every run names the exact method it used, and any change to the method is a reviewed diff in a pull request |
| **Not** `npx skills add` | Not used | It installs the newest upstream version from the network through an npm package, so two runs could use different methods, and the installer is extra unreviewed code |
| **Inactive by default** | The copy is **not** in `.claude/skills/`, so it does not load in everyday agent sessions | Loading it changes how agents answer security questions. That is a choice for the owner (section 9), not a side effect of storing it |
| **Activation** | Only on the audit machine, for an authorized run: checksums verified, then copied into that machine's user-level `~/.claude/skills/security-audit/` (section 6, step 2) | The target clone stays clean (the skill records whether the worktree is dirty), and the activation is visible and deliberate |
| **No GitHub Actions workflow** | None is added | An autonomous audit in CI would run without the owner's per-run approval, spend budget unattended, and write vulnerability details into a **public** repository's logs (D-022). It would need its own owner decision |
| **Licence** | MIT; development tooling only, never shipped | Recorded in [LICENSING.md](../../LICENSING.md) |

## 3. Requirements

### The audit machine

- A **disposable** Linux virtual machine on the owner's lab hardware (D-026): not a CI runner and not anyone's everyday computer. It is wiped after each run, once the results are archived.
- **Claude Code** with sub-agents (the skill runs many isolated agents in parallel). Model and account: the owner's choice when authorizing a run (section 4).
- **Node.js 18 or newer**, only for the skill's two validators. They run outside the sandbox and read only the run's own output files.
- **Git**, and the **Rust toolchain** pinned in `rust-toolchain.toml`, installed before the run.
- **Dependencies fetched beforehand.** The skill forbids builds that fetch anything. Before the run, the operator runs `cargo fetch --locked` for the main workspace, `desktop/` and `fuzz/`, outside the sandbox. The sandbox then gets the cargo cache read-only.

### The sandbox (the skill's universal execution safety, mandatory)

The audit agents read the source freely. **Septuvo's own code (builds, tests, fuzzers, labs) runs only inside an OS-enforced sandbox** that provides **all** of these controls:

| Control | Linux implementation (proposed) |
| --- | --- |
| No external network; an isolated loopback only when a check needs local client/server traffic | `bwrap --unshare-all` (new network namespace with loopback only) |
| An empty environment from an explicit allowlist; scratch-local `HOME`, temporary folders and caches | `bwrap --clearenv --setenv HOME <scratch>/home --setenv PATH … --setenv CARGO_TARGET_DIR <scratch>/target` |
| The target and toolchain read-only; writes only to the agent's `scratch/` | `--ro-bind` the clone, toolchain and cargo cache; `--bind` only `<scratch>` |
| Explicit CPU, memory, process, file-size, disk and wall-clock limits | `systemd-run --user --scope -p MemoryMax= -p TasksMax= -p CPUQuota= -p RuntimeMaxSec=` around `bwrap`, plus `prlimit --fsize=`; scratch on a size-limited filesystem |

**If any control cannot be enforced, Septuvo code is not run.** The lead stays `needs_validation`, with the missing control as its blocker; that is the skill's own rule.

**Checks that need root or real network namespaces** (the namespace labs in `tests/lab/`, the installer) cannot run inside an unprivileged `bwrap`. They may run only inside a second, **network-disabled throw-away VM** created for that check, where the VM boundary provides the controls above. Otherwise they stay `needs_validation`, with a validation plan the owner can run in the lab.

### Windows and macOS

Septuvo's Windows and macOS clients do not exist yet (D-014, M7). When they do, they are in scope **by source review** on any machine. **Running** them needs an equivalent sandbox on that operating system: Windows Sandbox with networking disabled, or a network-disabled macOS VM. Until such a sandbox is set up and approved, platform-specific behaviour stays `needs_validation`.

### Where results live

- The output folder is **outside the repository**, on the audit machine: `~/security-audit-skill/septuvo/run-<N>`.
- After the run it is archived **privately and encrypted**, with a SHA-256 manifest, to storage the owner chooses.
- **Nothing from a run is committed, pushed, pasted into an issue or pull request, or uploaded** before step 4 (section 7). The repository is public (D-022).

## 4. Costs

The skill counts **agent invocations**: about one hunter per coverage-ledger unit, plus reconnaissance, critics and one or two verifiers per candidate. The owner sets a **budget**, a maximum number of invocations. The skill refuses to start if the budget cannot cover reconnaissance, the critics and validation, and it reports an incomplete run rather than silently thinning the evidence.

**Rough estimate for Septuvo's current size.** About 25 entry surfaces (section 5), crossed with their trust boundaries and attack classes:

| Profile | Coverage units | Agent invocations (incl. reconnaissance, critics, verifiers) | Tokens, order of magnitude | Wall-clock |
| --- | --- | --- | --- | --- |
| `quick` (calibration, partial by design) | 15–30 | 30–60 | 2–6 M | hours |
| `standard` | 60–120 | 110–220 | 6–25 M | most of a day |
| `deep` (recommended for a release candidate) | 150–300 | 250–500 | 20–60 M | one to several days |

These are **estimates, not measurements**. Tokens per agent vary widely (roughly 30k–150k). The money cost depends on the model and on whether the run uses an API account (per-token prices) or a subscription plan (usage limits that can pause a long run); check current pricing when authorizing.

**Recommendation:** before the first full run, authorize one small `quick` run on one subsystem (for example the IPC framing in `crates/septuvo-ipc`) to measure the real cost per unit, then set the full run's budget from that. Plan for **at least two full runs per release candidate**, plus the post-fix run.

**Human time is a real cost too:** verifying confirmed findings and triaging `needs_validation` items (step 4) takes the owner's or a reviewer's time, roughly hours per confirmed finding.

## 5. Scope

The audit covers the whole repository at the authorized commit. These areas are named explicitly, with the skill's companion files that apply to each. Reconnaissance selects companions only for boundaries it actually finds.

| Area | Where (today) | Main questions | Companions |
| --- | --- | --- | --- |
| **Privileged service** `septuvod` | `crates/septuvo-daemon` | Root service: every request authorized after parsing; no path, link or race lets a local user write or delete as root; fails closed | Desktop/local IPC; resource exhaustion; memory safety (dependencies only: Septuvo code forbids `unsafe`) |
| **Privileged IPC** | `crates/septuvo-ipc`, control and status sockets | Group-based control, length-prefixed bodies, one request per connection; peer identity; status socket never changes anything | Protocols/RPC; desktop/local IPC |
| **Kill switch and firewall policy** | `crates/septuvo-policy` (nftables rendering), daemon engine | Fail closed; atomic replacement (see the N4-04 finding); Lockdown Mode; Local Network Access on hostile networks; IPv6 | Core attack classes; resource exhaustion |
| **DNS** | resolver file handling, DNS rules | No leak outside the tunnel; resolver-file backup and restore; DNS over HTTPS limits | Core attack classes |
| **Configuration handling** | `crates/septuvo-config` (TOML subset), service store, app drafts and preferences | Hostile files: bounds, injection into generated rules; atomic writes; permissions | Data isolation and lifecycle (import, export, backup) |
| **Desktop clients** | `desktop/` (Linux today; Windows and macOS when they exist) | Import of hostile files; file portal; export contents; untrusted service text; refuses root | Desktop/local IPC; client-side (only if a web view is ever added) |
| **Server administration** | `crates/septuvo-server` | Device records, revocation, file handling | Data isolation; core |
| **Installers and packaging** | `packaging/linux/`, systemd units, later the Rust installer (N4-09) | Privilege, file ownership, unit hardening, complete uninstall, boot ordering | Supply chain and release; desktop/local IPC (privileged helper) |
| **Dependencies and supply chain** | every `Cargo.lock`, `desktop/native-allowlist.txt`, the native-code gate, `.github/workflows/`, Dependabot, the pinned toolchain, this vendored skill | Pinning, two-week rule, CI permissions and triggers, script injection in workflows, artifact integrity | Supply chain and release |
| **Update mechanism** | D-042 (checking only, not built yet); signed updates at G3 | Whatever exists at the audited commit; design gaps go to `needs_validation`, not findings | Supply chain and release |
| **SVP protocol and cryptography** | after G1 only | Implementation bugs once code exists. **The audit never substitutes for the G1 specialist review** of the design | Protocols; memory safety |

**Out of scope:** live infrastructure, other people's systems, GitHub account and repository settings beyond the files in the repository, and the brand files. The skill itself forbids probing deployed endpoints.

## 6. Procedure

### Step 0: authorization (owner)

The owner authorizes each run separately and in writing, for example in a GitHub comment or an issue. The authorization names:
- the **exact commit**;
- the **profile** (`quick`, `standard` or `deep`);
- the **budget** in agent invocations;
- the **audit machine**;
- the **model/account**.

An agent never infers it from a merge or from this document.

### Step 1: the candidate

1. The candidate is a commit on `main` with **every CI check green**. Its full SHA is the only identifier; no release tag is created for this (tags need the owner's approval).
2. On the audit machine:
   - clone the repository;
   - `git checkout --detach <sha>`;
   - confirm `git status --porcelain` is empty;
   - run the existing tests locally, **inside the sandbox**, and record the results.

   A failing or dirty candidate is not audited.

### Step 2: the run

1. **Verify the method:** `cd third_party/cloudflare-security-audit-skill && sha256sum -c SHA256SUMS` (every line `OK`).
2. **Activate it on the audit machine only:** copy `third_party/cloudflare-security-audit-skill/skills/security-audit/` to `~/.claude/skills/security-audit/`.
3. **Prepare the sandbox** (section 3) and test it before the run:
   - a process inside cannot reach the network;
   - it cannot write outside scratch;
   - it sees no inherited environment variables;
   - it is stopped at its limits.

   Record the commands and limits used.
4. **Start Claude Code** in the clone, then ask for the full audit explicitly. For example:

   > Run a full security audit of this repository with the security-audit skill. Source ref: `<sha>`. Profile: `deep`. Budget: `<N>` agent invocations. Output directory: `~/security-audit-skill/septuvo/run-<N>`. Prior runs: `<paths, or none>`. Scope and sandbox: `docs/security/AI_ASSISTED_AUDIT.md` sections 3 and 5. Run Septuvo code only through the sandbox wrapper `<command>`; if a check cannot run there, record it as needs_validation.

5. The run ends in one of the skill's two terminal states:
   - all reports written and both validators passing;
   - or `run_status: "incomplete"`, with the reason disclosed.

### Step 3: the results

These stay in the private output folder:
- `run-metadata.json`, `architecture.md`, `coverage-ledger.json`;
- `findings.json`;
- `REPORT.md`, `FINDINGS-DETAIL.md`, `NEEDS-VALIDATION.md`.

Then:
- Archive them encrypted, with a SHA-256 manifest.
- Write a short, **non-sensitive run record** for the owner: commit, profile, budget used, counts per verdict and severity, and the coverage statement including what was **not** covered. It contains no exploit details.

### Step 4: verify before remediation

**No fix starts from an unverified finding.**

- **`confirmed`:** each finding is re-checked against the source by a **different** session or a human, and reproduced only inside the sandbox. The owner decides whether it is accepted, disputed (with evidence) or out of scope.
- **`needs_validation`:** each item is triaged. Either the owner runs its safe validation plan in the lab, or it becomes a tracked open risk.
- **Accepted vulnerabilities** are recorded privately as a **GitHub draft security advisory**, the repository's private reporting channel ([SECURITY.md](../../SECURITY.md)). They are **not** recorded as public issues.

### Step 5: remediation

Fixes go through the normal owner-reviewed process: a pull request, CI and the owner's merge.

- **One finding per pull request** where practical. Each fix includes a **regression test** that fails before the fix.
- **Sensitive findings** are fixed in the advisory's **temporary private fork**. Public pull-request text describes the change without exploit details until the fix is merged.
- Agents never merge, and never publish advisory details.

### Step 6: re-audit

After the owner merges the fixes:
- run a **fresh** audit on the new commit (steps 0–3 again, with a new authorization);
- give it the earlier run folder as a **prior run**.

The skill then:
- re-validates every earlier finding whose code changed;
- carries forward only unchanged, still-valid evidence;
- turns earlier gaps into current work.

### Step 7: compare and record

Write a comparison for the owner, kept private until the fixes are public:

- **Fixes verified:** each earlier `confirmed` fingerprint is now absent or rejected, **because the relevant source changed**, and its regression test passes.
- **Regressions:** new findings in changed code, investigated first.
- **Unchanged findings and open `needs_validation` items:** still open.
- **Coverage:** what both runs together covered, and what neither did.
- **Remaining risks**, stated plainly. A run with no findings is reported as "no findings in the covered units", never as "secure".

Repeat steps 6–7 until the owner is satisfied, and **before every release candidate**.

## 7. Handling vulnerability details

- **The repository is public** (D-022). Raw outputs, exploit steps and proof-of-concept inputs never go into commits, pull requests, issues, discussions, CI logs or chat transcripts that others can see.
- **Private channel:** GitHub private vulnerability reporting and draft security advisories ([SECURITY.md](../../SECURITY.md)).
- **Publication:** the owner decides whether and when to publish, after fixes are merged. Nothing is published automatically.
- **Secrets:** the skill's rules already forbid real credentials, live endpoints and ambient environment capture; runs use dummy data only.

## 8. Relationship to the gates and reviews

| Gate | What this audit can contribute | What it cannot do |
| --- | --- | --- |
| **G1** (protocol design review) | Findings in protocol-adjacent code, as input to the reviewers | Replace the qualified specialist review of the SVP design and cryptography |
| **G2** (lab qualification) | Findings and validation plans for the lab | Replace the real two-node lab evidence |
| **G3** (production release) | Repeated runs on release candidates, with fix verification, as one input | Replace the **independent** implementation and security assessment that G3 requires |

Audit results never let anything be described as "audited" or "secure" (D-002, [SECURITY.md](../../SECURITY.md)).

## 9. Decisions for later (not needed now)

1. **Audit machine and sandbox:** approve the VM and the `bwrap` + `systemd-run` setup in section 3, or name another.
2. **The first run:** commit, profile, budget, model and account (step 0). Recommended: a `quick` calibration run on one subsystem first.
3. **Guidance mode in everyday work** (optional): whether to also activate the skill project-locally (`.claude/skills/security-audit/`) so agents use its methodology for focused security questions during development. It never starts a full audit by itself. Off until the owner decides.
4. **Any CI automation:** not proposed.
