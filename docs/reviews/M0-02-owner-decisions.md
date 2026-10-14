# M0-02 — Owner product decisions

| | |
| --- | --- |
| **Task** | M0-02 — Resolve baseline product decisions ([TASK_CATALOG.md](../TASK_CATALOG.md#m0-02--resolve-baseline-product-decisions)) |
| **Work order** | [Issue #18](https://github.com/neosmith20/Septuvo/issues/18) — approved by the owner: [`APPROVED WORK ORDER M0-02`](https://github.com/neosmith20/Septuvo/issues/18#issuecomment-6162237599) (2026-10-14 UTC) |
| **Owner answer** | [`Q1–Q9: as recommended`](https://github.com/neosmith20/Septuvo/issues/18#issuecomment-6162235869) (2026-10-14 UTC) |
| **Recorded by** | Claude (agent). This file records the owner's answers; it makes no decision of its own. |

The owner answered all nine questions with one comment, "Q1–Q9: as recommended". Each decision below is therefore the **recommended option exactly as written in the approved work order**; nothing was added or filled in. No cryptographic choice was made: D-010, D-011, D-012 and D-024 remain **candidates** until G1, and D-013 remains **blocked**.

## Decisions

| Q | Question | Owner decision (as recommended) | Recorded as |
| --- | --- | --- | --- |
| Q1 | First lab platform | Linux x86_64, Ubuntu Server 24.04 LTS, in isolated virtual machines and network namespaces | New **D-026** |
| Q2 | Order of operating systems after the Linux lab | **A** — Linux → Windows → macOS → Android → iOS | **D-014** updated |
| Q3 | Language exceptions | Yes — all Septuvo-owned security logic stays in Rust; small amounts of platform glue (Swift/Kotlin/C/Objective-C) only where an operating system requires them, each listed in a platform inventory | **D-005** updated |
| Q4 | Version 1 user journeys | **A and B** — A) reach my home network (NAS/Jellyfin) securely from anywhere; B) protect my traffic on untrusted Wi-Fi through my own server. C (small-business site-to-site) and D (multi-tenant provider, M9) are not version 1 | New **D-027** |
| Q5 | Self-hosting | Yes — standalone install with no account, no cloud and no Septuvo-run service required; optional relay or coordination only by explicit opt-in | **D-007** updated |
| Q6 | Risk assumptions | Yes — the attacker list and the "out of scope" list in [SECURITY_MODEL.md](../SECURITY_MODEL.md) are the baseline threat assumptions for M1 | New **D-028** |
| Q7 | Parallel non-crypto track before G1 | Yes — after G0, a separate lab-only track: Rust workspace skeleton and CI, Linux route / firewall / DNS / kill-switch policy prototypes in network namespaces, and CLI/UI scaffolding. **Never** any SVP handshake, packet format, key handling or other cryptography before G1. The roadmap and task-catalog changes for this track are a follow-up task, not part of M0-02 | New **D-029** |
| Q8 | Decision labels (finding A-06) | Yes — four states only: **Owner decided** (with date and link), **Candidate**, **Open**, **Blocked** | Applied to every row in [DECISIONS.md](../DECISIONS.md) |
| Q9 | Confirm existing directions | Yes, all — D-003, D-004, D-005, D-006, D-007, D-008, D-015, D-017, D-019 and D-020 become *Owner decided 2026-10-14*; D-009 (scale) and D-018 (performance) become owner-decided **goals that must be measured before any claim** | Labels updated in [DECISIONS.md](../DECISIONS.md) |

## How Q8 was applied to rows not named in Q9

- D-001 and D-002 were already "Owner confirmed"; they are relabelled **Owner decided**. The original confirmation date was not recorded and is shown as such; D-002's secondary line keeps its 2026-10-14 date.
- D-010, D-011, D-012 and D-024 keep the label **Candidate** with their existing "not approved" wording.
- D-013 keeps **Blocked**; D-016 (name/domain/trademark clearance) is relabelled **Open**.
- D-021, D-022, D-023 and D-025 were already "Owner decided 2026-10-14"; unchanged.

## Follow-ups created by these decisions (not done here)

- **Q7:** the roadmap and task catalog need a separate, owner-approved task defining the parallel non-crypto track (task IDs, entry after G0, lab limits, and the explicit "no cryptography before G1" boundary).
- **Q1:** M0-03 plans the actual lab resources (hosts, isolation, network namespaces) for this platform.
- Open questions 1–8 in DECISIONS.md remain open for M1/M2.
