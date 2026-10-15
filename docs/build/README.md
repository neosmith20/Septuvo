# Building and testing the Septuvo Rust workspace

**State:** development preview from track N1 (non-crypto foundation, [D-029](../DECISIONS.md)). This code **cannot** connect, carry traffic, encrypt or protect anything. It has no SVP handshake, packet format, key handling or tunnel; those wait for gate G1.

## What is here

| Path | What it is |
| --- | --- |
| [`Cargo.toml`](../../Cargo.toml) | Workspace: shared version, edition 2024, Apache-2.0 licence, `unsafe` code forbidden, Clippy pedantic lints |
| [`rust-toolchain.toml`](../../rust-toolchain.toml) | Pinned Rust release (1.99.0, the current stable on 2026-10-14) with `rustfmt` and `clippy` |
| [`crates/septuvo-policy`](../../crates/septuvo-policy) | Route and LAN-exception policy model and the nftables kill-switch generator: pure logic, no operating-system calls (N1-02, N1-03) |
| [`crates/septuvo-cli`](../../crates/septuvo-cli) | The `septuvo` command-line tool; for now only `help` and `version` (commands arrive in N1-04) |
| [`.github/workflows/rust.yml`](../../.github/workflows/rust.yml) | CI: no-third-party-crate check, format, lint with warnings as errors, tests; plus the namespace kill-switch lab |
| [`tests/lab/namespaces/`](../../tests/lab/namespaces) | Kill-switch, DNS and IPv6 leak tests in throw-away network namespaces: see [lab.md](lab.md) (N1-03) |

**No third-party crates.** Everything uses the Rust standard library only. CI fails if `Cargo.lock` lists a crate from a registry. Adding any dependency needs an owner-approved work order and a dependency review (D-015).

## Build and test locally

1. Install Rust with [rustup](https://rustup.rs/). The first `cargo` command reads `rust-toolchain.toml` and fetches the pinned release.
2. From the repository root, run the same checks as CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

3. Try the tool (it only prints help and its version):

```sh
cargo run -p septuvo-cli -- help
```

Build output goes to `target/`; never commit it (`.gitignore` excludes it).

## Updating the toolchain

Toolchain updates are deliberate: change `channel` in `rust-toolchain.toml` in its own pull request, record the new release and run CI. Nothing updates it automatically.
