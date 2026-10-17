# Building and testing the Septuvo Rust workspace

**State:** development preview from tracks N1 and N2 (non-crypto foundation, [D-029](../DECISIONS.md)). This code **cannot** connect, carry traffic, encrypt or protect anything. It has no SVP handshake, packet format, key handling or tunnel; those wait for gate G1.

## What is here

| Path | What it is |
| --- | --- |
| [`Cargo.toml`](../../Cargo.toml) | Workspace: shared version, edition 2024, `unsafe` code forbidden, Clippy pedantic lints. Each crate declares its own licence ([LICENSING.md](../../LICENSING.md)) |
| [`rust-toolchain.toml`](../../rust-toolchain.toml) | Pinned Rust release (1.99.0, the current stable on 2026-10-14) with `rustfmt` and `clippy` |
| [`crates/septuvo-policy`](../../crates/septuvo-policy) | Route and LAN-exception policy model and the nftables kill-switch generator: pure logic, no operating-system calls (N1-02, N1-03) |
| [`crates/septuvo-cli`](../../crates/septuvo-cli) | The `septuvo` command-line tool and the four-stage connection status model (D-020): `help`, `version`, `status`, `connect`, `disconnect` (asking `septuvod`), `policy check FILE [--dst ADDRESS] [--path up\|down]` (N1-01, N1-04, N2-02) |
| [`crates/septuvo-config`](../../crates/septuvo-config) | Configuration format (strict TOML subset), parser and validation for device and server files, and the `septuvo-config check` tool: see [config.md](config.md). PolyForm Noncommercial (N2-01) |
| [`crates/septuvo-ipc`](../../crates/septuvo-ipc) | The local control protocol between `septuvo` and `septuvod`. Apache-2.0 (N2-02) |
| [`crates/septuvo-daemon`](../../crates/septuvo-daemon) | `septuvod`, the privileged service (lab only): see [daemon.md](daemon.md). PolyForm Noncommercial (N2-02) |
| [`crates/septuvo-server`](../../crates/septuvo-server) | `septuvo-server`, local server administration of device records (no enrollment, keys or listener before G1): see [server-admin.md](server-admin.md). PolyForm Noncommercial (N2-04) |
| [captive-portal.md](captive-portal.md) | Captive-portal sign-in (N3-02): helper-only design, RFC 8910/8908 and DoH evaluation, unsupported legacy portals, phased plan |
| [`packaging/linux/`](../../packaging/linux) | Linux installer, uninstaller and hardened systemd units (lab namespace only): see [install.md](install.md). PolyForm Noncommercial (N3-01) |
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

3. Try the tool. It cannot connect anything yet: `status` reports the four checks as unknown, and `policy check` validates a policy file and explains it:

```sh
cargo run -p septuvo-cli -- help
cargo run -p septuvo-cli -- status
cargo run -p septuvo-cli -- policy check tests/lab/namespaces/policies/journey-a.policy --dst 192.168.50.10
```

Build output goes to `target/`; never commit it (`.gitignore` excludes it).

## Updating the toolchain

Toolchain updates are deliberate: change `channel` in `rust-toolchain.toml` in its own pull request, record the new release and run CI. Nothing updates it automatically.
