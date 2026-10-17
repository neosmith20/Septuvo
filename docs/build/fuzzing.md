# Fuzz testing (N3-03)

**What:** systematic testing of every Septuvo parser with millions of generated, mutated and hostile inputs, using only the Rust standard library.

**Why:** configuration files, the local control protocol, device records and system files are all **hostile input** (AGENTS.md). Hand-written tests check the cases someone thought of. Fuzzing checks the ones nobody did.

## Method

**The generator** is one shared file, [`crates/septuvo-policy/tests/support/fuzzer.rs`](../../crates/septuvo-policy/tests/support/fuzzer.rs). It is Apache-2.0, included by path into the other crates' fuzz tests: Apache code may be used by the PolyForm crates (D-033), not the other way round.

- **Test code only.** Its random numbers come from SplitMix64: fast and reproducible, and **never** used for anything security-related.
- **Every case comes from one 64-bit case seed,** so any failure can be replayed exactly.
- **Each case is one of:**
  - random bytes;
  - a string of the target's dictionary tokens (keywords, limits, special addresses, escape sequences);
  - a **mutation** of a seed input: bit flips, inserted, deleted, duplicated or spliced chunks, inserted tokens and boundary numbers;
  - a **structure-aware** input from the target's own generator, mostly valid, so the deeper code is reached.
- **Each target reports how many inputs were accepted.** A target that never gets past the first syntax check is visible at once.

**Properties checked:**

| Property | Meaning |
| --- | --- |
| No panic | Any input, however broken, gives a clean error |
| Bounded output | Error messages, rulesets, scripts and responses stay within fixed limits |
| Round trip | Where a format can be written back, writing and re-reading what was accepted gives the same thing |
| Rejecting changes nothing | A refused request or operation leaves the state exactly as it was |
| Cross-checks | Things that must agree do agree, for example a device configuration the validator accepts always renders to a kill-switch ruleset |

## Targets

| Crate | Target | Parser | Properties beyond "no panic" |
| --- | --- | --- | --- |
| `septuvo-policy` | `fuzz_policy_file` | Policy file format | Round trip; deciding any address never panics; bounded errors with valid line numbers |
| | `fuzz_prefix` | Address prefixes | Round trip; a prefix contains its own address; length within the family |
| | `fuzz_nft_render` | nftables renderer, with hostile interface names, ports, resolvers and helper IDs | Only validated values reach the ruleset; printable output; **no injected statements** (semicolons only from fixed templates); bounded size; captive rules always scoped to the helper; root never the helper; bounded captive-portal script |
| `septuvo-config` | `fuzz_toml_subset` | The TOML subset | Deterministic; every limit holds; **round trip** through a test-only writer; error positions in range |
| | `fuzz_device_schema` | `septuvo.toml` | Deterministic; invariants; **every accepted configuration renders a kill switch** |
| | `fuzz_server_schema` | `septuvo-server.toml` | Invariants (address families, absolute data directory, retention limit) |
| `septuvo-ipc` | `fuzz_ipc_request` | Control requests | **Only the canonical encoding of a command is accepted**; captive minutes in range |
| | `fuzz_ipc_response` | Control responses | Round trip; field limit |
| | `fuzz_ipc_builder` | The response builder (any keys, values, codes, messages) | **A built response always parses and never exceeds the size limit** |
| `septuvo-server` | `fuzz_device_records` | `devices.db` | Round trip and stable encoding; valid names and in-range addresses; printable, bounded errors |
| | `fuzz_record_operations` | Sequences of add, revoke and remove with hostile names | **A refused operation changes nothing**; every state still round-trips; printable errors |
| `septuvo-daemon` | `fuzz_daemon_settings` | `septuvod.toml` | Round trip; root never the captive helper |
| | `fuzz_resolver_file` | `resolv.conf` (network resolvers) | At most 4; never loopback, unspecified or multicast; always from a `nameserver` line; always a valid captive-portal script |
| | `fuzz_account_files` | `/etc/passwd`, `/etc/group`, `/proc/self/status` | IDs only from the line naming exactly that account |
| | `fuzz_control_requests` | Hostile bytes on the real socket handler, both sockets | Only well-formed requests reach the service; a well-formed one always does |
| | `fuzz_status_socket_changes_nothing` | Change commands sent on the status socket | Always refused; the kill switch never changes |

## Running

```sh
cargo test --workspace --test fuzz                     # 500 cases per target (part of every test run)
SEPTUVO_FUZZ_ITERS=100000 cargo test --release --workspace --test fuzz -- --nocapture
SEPTUVO_FUZZ_SECONDS=1800 cargo test --release -p septuvo-config --test fuzz -- fuzz_toml_subset --exact --nocapture
```

| Variable | Meaning | Default |
| --- | --- | --- |
| `SEPTUVO_FUZZ_ITERS` | cases per target | 500 |
| `SEPTUVO_FUZZ_SECONDS` | run each target for this long instead | unset |
| `SEPTUVO_FUZZ_SEED` | base seed (decimal or `0x` hex) | fixed per target |
| `SEPTUVO_FUZZ_REPLAY` | run exactly one case seed | unset |

**When a property fails,** the runner prints the target, the **case seed** and the input, for example:

```text
FUZZ FAILURE target=fuzz_ipc_builder case_seed=0x35f6d570c8fcedeb
  input: \xd9\xcaL\xba\xeb/$\x1e
  replay: SEPTUVO_FUZZ_REPLAY=0x35f6d570c8fcedeb cargo test --test fuzz fuzz_ipc_builder -- --nocapture
```

The fix then gets a regression test that names the seed.

## In CI ([`.github/workflows/fuzz.yml`](../../.github/workflows/fuzz.yml))

- **Every pull request:** a short run, 20,000 cases per target with a fresh seed (the workflow run ID). The summary lists every target's result.
- **Weekly (Mondays), on demand, or on a pull request labelled `fuzz-long`:** a long run with one parallel job per target, 30 minutes each by default. The on-demand run takes a `minutes` input.

## Bugs found

**Three real bugs, all fixed with regression tests:**

| Where | Bug | Fix |
| --- | --- | --- |
| `septuvo-ipc`, `Response::with` (seed `0x35f6d570c8fcedeb`) | A response built with many long fields could encode to **4,298 bytes, more than the 4,096 the client accepts**. The client would then show "invalid answer from the Septuvo service". Not reachable with today's status fields, but nothing prevented it | Fields that would not fit are dropped. A missing field already reads as "unknown", never as passed, so this fails safe. An error with an empty message also no longer gains a trailing space when re-encoded |
| `septuvo-config`, device schema (found in CI, seed `0xc8c5a99ba1125cd2`) | The validator accepted a tunnel interface name starting with `-` (`-1septuvo0`), but the kill-switch renderer refuses one: **a configuration that passed `septuvo-config check` would fail at connect**. The two rules had drifted apart when the renderer was hardened in #60 | The validator now calls the renderer's own check, `nft::valid_interface`, so the rules cannot drift again |
| `septuvo-server`, `Records::revoke` / `remove` (seed `0x496239978f4b4369`) | Errors for impossible device names **repeated the name, control characters included**. Printed to a terminal, such a name could carry escape sequences. The command-line tool already filtered them, but the library did not | An invalid name is refused without being repeated |

**Mistakes in the new tests themselves, also found by fuzzing and corrected (not product bugs):**
- a semicolon count that forgot one fixed comment line;
- an address comparison that ignored upper-case IPv6;
- an over-strict generator;
- a settings round-trip writer that did not escape quotes in paths.

**A flaw in the first version of this workflow, also caught in this PR:** it piped `cargo test` through `tee` without `pipefail`, so **a failing run was reported as success**. A long-run job that "finished" after 12 seconds exposed it. Both jobs now:
- set `pipefail`;
- fail if the target did not run;
- turn failure lines into readable annotations.

## Not included

**Coverage-guided fuzzing** (`cargo-fuzz` / libFuzzer, or AFL) would find deeper bugs faster, because it watches which code each input reaches. It needs a third-party crate and a nightly toolchain, so it is an **owner decision on dependencies (D-015)**.

**To add it later:**
1. Approve `cargo-fuzz` (development-only, never shipped).
2. Add a `fuzz/` directory with one `fuzz_target!` per row of the table above, calling the same property functions.
3. Use the seed inputs here as the starting corpus.
4. Run it in the weekly job on nightly.

The standard-library runner stays as the per-pull-request check.

## Results

**Local, 2026-10-17:** 2 minutes per target, base seed 20261017. Linux x86_64 with 2 CPUs (targets shared them), Rust 1.97, release build. **All properties held for every target: 257,155,030 cases in total.**

| Target | Cases | Accepted |
| --- | ---: | ---: |
| `fuzz_account_files` | 12,239,397 | 2,443,199 |
| `fuzz_control_requests` | 2,558,032 | 285,235 |
| `fuzz_daemon_settings` | 18,406,716 | 2,336,188 |
| `fuzz_device_records` | 11,893,402 | 656,137 |
| `fuzz_device_schema` | 2,094,224 | 103,401 |
| `fuzz_ipc_builder` | 371,747 | 371,747 |
| `fuzz_ipc_request` | 49,554,057 | 3,527,301 |
| `fuzz_ipc_response` | 11,328,776 | 197,821 |
| `fuzz_nft_render` | 966,555 | 438,761 |
| `fuzz_policy_file` | 9,698,138 | 720,312 |
| `fuzz_prefix` | 63,175,410 | 11,627,086 |
| `fuzz_record_operations` | 449,695 | 13,979 |
| `fuzz_resolver_file` | 18,522,872 | 4,216,814 |
| `fuzz_server_schema` | 7,149,485 | 293,675 |
| `fuzz_status_socket_changes_nothing` | 40,470,762 | 8,503,985 |
| `fuzz_toml_subset` | 8,275,762 | 1,252,269 |

**CI long run, 2026-10-17** (workflow run 39020159652, commit `cbf97eb`, pinned Rust toolchain, base seed = the run ID): **all 16 targets ran for 30.0 minutes each, and every property held.**
- Each job fails unless its target reports "all properties held".
- The per-target case counts are in that run's job summaries.

**The earlier CI runs in the same pull request found the device-schema bug above.**
