# Septuvo licensing

**Owner decision D-033 (2026-10-15).** This page explains which license covers which part of Septuvo. It is a plain-language summary prepared by the project, **not legal advice**. The license texts themselves are what apply, and the open legal questions in section 8 still need review by a qualified attorney.

## 1. The boundaries

| Part | License | What it means |
| --- | --- | --- |
| **Septuvo Protocol (SVP)**: the specification and its open reference implementation | [Apache License 2.0](LICENSE) | Anyone may use, modify, distribute, implement and commercialize the protocol, without paying Septuvo |
| **Septuvo VPN** and **Septuvo Server**: application and server code written from now on | [PolyForm Noncommercial 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.md), plus a separate written commercial agreement | Public and fully inspectable (**source-available**, not "open source"). Free for noncommercial use. Commercial use needs a written agreement with the Septuvo rights holder (section 4) |
| **Septuvo name, brand, logos and original artwork** | Rights reserved; see [brand/NOTICE.md](brand/NOTICE.md) | No trademark or brand permission is granted by any code license. Earlier grants are kept (section 3) |
| **Everything already published** (all files up to and including this change) | Apache License 2.0, as published | Those grants are permanent and are **not** withdrawn (section 3) |

### Current classification of the Rust crates

| Crate | License | Component | Why |
| --- | --- | --- | --- |
| `crates/septuvo-policy` | Apache-2.0 | Open reference implementation | Policy model and kill-switch generator; published under Apache-2.0 since N1-02 |
| `crates/septuvo-cli` | Apache-2.0 | Open reference implementation | Reference command-line tool and status model; published under Apache-2.0 since N1-01 |
| `crates/septuvo-config` | PolyForm Noncommercial 1.0.0 | Septuvo VPN and Server | Configuration format and validation (N2-01) |
| `crates/septuvo-ipc` | Apache-2.0 | Open reference implementation | Local control protocol between the `septuvo` tool and the service; Apache-2.0 because the Apache-2.0 CLI depends on it (N2-02, approved in the N2 work order) |
| `crates/septuvo-daemon` | PolyForm Noncommercial 1.0.0 | Septuvo VPN | Privileged service `septuvod` (N2-02) |
| `crates/septuvo-server` | PolyForm Noncommercial 1.0.0 | Septuvo Server | Server administration: device records (N2-04) |
| `packaging/linux/` (not a crate) | PolyForm Noncommercial 1.0.0 | Septuvo VPN | Linux installer, systemd units and templates (N3-01); carries its own `LICENSE.md` and `NOTICE` |

New Septuvo VPN or Septuvo Server application code goes into **new crates** classified as PolyForm Noncommercial in [`licensing.toml`](licensing.toml). Whether the two existing crates should instead move to PolyForm for future changes is owner question **L6** (section 8).

## 2. Who holds the rights

The **Septuvo rights holder** is the owner of this repository: the person behind the GitHub account **neosmith20**, who directs all work and reviews and merges every change (D-023, D-031). That person's full legal name, or a company if one is formed, must be filled in before any commercial agreement is signed (question **L1**).

| Who contributed (from the Git history) | Status |
| --- | --- |
| neosmith20 (the owner) | Rights holder |
| AI coding agents (Claude, and earlier ChatGPT/Codex) working through the owner's account at the owner's direction (D-023) | The owner is treated as rights holder. **How far AI-generated material is protected by copyright at all is unsettled** (question **L2**) |
| `aikido-autofix[bot]` (7 commits to repository scripts) and `dependabot[bot]` (4 version bumps) | Accepted by the owner's merge under Apache-2.0. Ownership of bot output is question **L3** |

## 3. What stays the same

- **Every Apache-2.0 grant already made stays in force.** Everything published before this change remains available under Apache-2.0 for those versions, including:
  - all files in this repository;
  - pushed branches, such as the N1-03/N1-04 branch;
  - the brand files.

  Nobody needs permission to keep using them under Apache-2.0. Nothing here relicenses published code retroactively, and Git history is not rewritten.
- **The root [`LICENSE`](LICENSE) stays the unmodified Apache License 2.0.** It continues to cover the SVP specification, the reference implementation, the documentation and the repository tooling. CI checks that it is never altered.
- **Third-party licenses are unaffected.**
  - No third-party code is bundled today.
  - The brand kit names the Inter typeface but does not include it (Inter has its own license).
  - GitHub Actions used in CI are not distributed.
- **All G0–G4 security gates and requirements are unchanged.**

## 4. Using Septuvo VPN and Septuvo Server

Once PolyForm-licensed application or server code exists:

- **Free (PolyForm Noncommercial):**
  - personal use, study, hobby projects, self-hosting at home, research, experiments and testing without commercial application;
  - use by charities, schools, public research, public safety or health, environmental and government organizations (as the license defines them).
- **Needs a separate written commercial agreement:**
  - selling or reselling it;
  - including it in a paid product;
  - running it as part of a commercial VPN service;
  - any other commercial use, including use inside a business.
- **To ask about a commercial license:** contact the rights holder through the GitHub account **neosmith20**. A dedicated contact address is question **L4**.

Every copy of PolyForm-licensed code must keep the license text and the `Required Notice:` line in that crate's `NOTICE` file, for example:

```text
Required Notice: Copyright 2026 the Septuvo rights holder (GitHub: neosmith20). Commercial use requires a separate written license agreement.
```

## 5. Contributions

- **Apache-2.0 parts** (SVP specification, reference implementation, documentation, tooling): contributions are accepted under Apache-2.0, as section 5 of that license provides.
- **PolyForm parts** (Septuvo VPN and Septuvo Server application code): outside contributions are **not accepted** until a contributor agreement, drafted by counsel, gives the rights holder the right to offer them under commercial licenses as well (question **L5**). The owner's own work, and work done for the owner, is not affected.

## 6. How the rules are enforced

[`scripts/check_licensing.py`](scripts/check_licensing.py) runs on every pull request. It fails when:

- the root `LICENSE` or the PolyForm text is altered;
- the workspace sets a default crate license, so new crates would inherit it silently;
- a crate is not classified in [`licensing.toml`](licensing.toml), or its `Cargo.toml` license does not match;
- a PolyForm crate could be published to crates.io, or lacks its license copy and `Required Notice:`;
- an Apache-2.0 crate depends on a PolyForm crate. The open protocol must never need restricted code.

Adding a crate therefore always means choosing and recording its license.

## 7. Why PolyForm Noncommercial (evaluation)

| Option | Fits the owner's goals? | Notes |
| --- | --- | --- |
| **PolyForm Noncommercial 1.0.0** (chosen) | Yes | Standard, short, readable. Free for personal and noncommercial-organization use. All commercial use, including use inside a business, needs a separate agreement. Includes a patent license. Not sublicensable, so resellers need an agreement |
| PolyForm Small Business 1.0.0 | Partly | Free for small businesses too, which would give away commercial use the owner wants to license |
| PolyForm Shield 1.0.0 | No | Only forbids competing with the licensor; most commercial use would be free |
| Business Source License 1.1 | Partly | Converts to open source after a set date; adds a time-limited model the owner did not ask for |
| Elastic License 2.0 | No | Mainly forbids offering the software as a managed service; resale and bundling would be free |
| AGPL-3.0 with a commercial option | No | Open source: commercial use would be allowed as long as source is shared |

**Consequence to know:** PolyForm-licensed parts are **source-available**, not "open source" under the OSI definition. Project materials must not call Septuvo VPN or Septuvo Server "open source". SVP remains open source (Apache-2.0).

## 8. Open legal questions for the owner (and counsel)

| # | Question | Why it matters |
| --- | --- | --- |
| L1 | Who exactly is the rights holder: you as an individual, or a company (for example an LLC)? If a company is formed later, rights must be assigned to it | Notices and commercial agreements must name the right legal person |
| L2 | How much of the AI-generated code and text is protected by copyright? Current U.S. guidance protects human authorship, including human selection, arrangement and modification, but not material generated by AI alone | Affects how enforceable the PolyForm terms and commercial licenses are for AI-written code. Keeping a record of human direction and edits helps; counsel should advise |
| L3 | Ownership of bot-generated changes (Aikido autofix, Dependabot) | Those changes are in Apache-licensed tooling, so the impact is small; check each service's terms |
| L4 | A contact address for commercial licensing inquiries | So companies know where to ask |
| L5 | A contributor agreement (CLA or assignment) for outside contributions to PolyForm parts | Without it, outside code could not be included in commercial licenses |
| L6 | Should `septuvo-policy` and `septuvo-cli` stay Apache-2.0 (recommended: they are small, published and forkable anyway), or move to PolyForm for future changes? | Published Apache versions stay Apache either way; moving mainly adds complexity |
| L7 | The brand files were published under Apache-2.0 (copyright only; Apache-2.0 grants no trademark rights). Trademark clearance is still pending (D-016) | Earlier copies of the artwork remain usable under Apache-2.0 copyright terms; the name and logos still cannot be used to identify other products |
| L8 | A written commercial license agreement template | Needed before the first commercial deal |
| L9 | Before binary releases (G3): license notices for the Rust standard library compiled into binaries, and export-control review for encryption software | Release-time obligations; not needed yet |

None of these blocks this change. They need answers before the first commercial agreement or binary release.
