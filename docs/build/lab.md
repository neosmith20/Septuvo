# Kill-switch lab in network namespaces (N1-03)

**What this proves:** on Linux, the firewall rules that Septuvo generates from a policy stop protected IPv4, IPv6 and DNS traffic from leaving outside the VPN path. They do so when the path is down and when a hostile network pushes routes around it. Approved exceptions keep working, and the rules apply, replace and roll back cleanly.

**What this does not prove:** that any tunnel is secure. There is **no SVP, no encryption and no real tunnel** here. A plain virtual interface named `septuvo0` stands in for the tunnel so the lab can see where packets go. Cryptographic work waits for gate G1.

## How it works

| Piece | Where | Role |
| --- | --- | --- |
| Ruleset generator | [`crates/septuvo-policy/src/nft.rs`](../../crates/septuvo-policy/src/nft.rs) | Turns a validated policy into an nftables script. Pure text, unit-tested |
| Generator tool | [`crates/septuvo-policy/examples/render_nft.rs`](../../crates/septuvo-policy/examples/render_nft.rs) | `render_nft <policy> <tunnel-interface> [endpoint ...]` prints the script; `--remove` prints the removal script |
| Lab harness | [`tests/lab/namespaces/run.sh`](../../tests/lab/namespaces/run.sh) | Builds three throw-away network namespaces, applies the rules, sends probes, reports PASS / FAIL / SKIP |
| Probe | [`tests/lab/namespaces/probe.py`](../../tests/lab/namespaces/probe.py) | Sends a unique token, or listens and logs every token received |
| Lab policies | [`tests/lab/namespaces/policies/`](../../tests/lab/namespaces/policies) | Journey B with one approved local device, journey A (split), overlapping subnets |

The three namespaces:

- **C, the device:** gets the kill switch. Its Wi-Fi is `192.168.1.10` on a café network `192.168.1.0/24`.
- **U, the café network:** the router (`192.168.1.1`), another local host (`.20`), an approved device (`.30`), a local DNS server (`.53`), the Septuvo Server endpoint (`203.0.113.7`) and "the internet" (`198.51.100.20`). It also has a device using a home-network address (`192.168.50.10`), so leaks of home traffic are visible.
- **T, the far end of the stand-in tunnel:** the same internet and home addresses, so a tunnelled packet and a leaked packet are told apart by **where the token arrives**.

Each case expects **tunnel**, **underlay** (café network) or **blocked** (nowhere). Control cases first run **without** the kill switch and must show the leak, which proves the detector works.

## What is tested

| Area | Cases | Requirement |
| --- | --- | --- |
| Detector controls (no kill switch: must leak) | C01–C04 | — |
| Full tunnel: internet, DNS through tunnel | B01–B03 | NP-S1 |
| DNS to the café resolver blocked (UDP 53, TCP 853, IPv6), even to an approved device | B04–B06, B09, B24 | NP-D1 |
| Approved local device direct, others blocked | B07, B08, B10, B25, B28 | NP-S4, NP-E7 |
| Underlay exceptions: server endpoint only on its port, DHCPv4/v6, IPv6 neighbour discovery | B08, B11–B15, B26, B27 | NP-U1–NP-U3 |
| Hostile network pushes a more specific route (for example DHCP option 121) | B16, B17 | NP-E1 |
| Kill switch with the path down, IPv4/IPv6/TCP, no DNS fallback | B20–B23 | NP-E1, NP-E3, NP-6.1 |
| Journey A split tunnel: home through tunnel, fails closed; internet direct by design | A01–A12 | NP-S2 |
| Overlapping subnets: the tunnel wins; never delivered to the café | O01–O11 | NP-S5 (Q5) |
| Atomic apply, full replacement, clean removal, normal networking restored | R01–R05 | NP-E6, Q2 |

## Run it

On a **disposable** Ubuntu 24.04 machine or VM, as root. It changes nothing outside its own namespaces, which it deletes on exit even if a test fails:

```sh
sudo apt-get install -y nftables iproute2
cargo build -p septuvo-policy --example render_nft --locked
sudo tests/lab/namespaces/run.sh target/debug/examples/render_nft
```

CI runs the same thing on every pull request (job **lab (kill switch in network namespaces)** in [`rust.yml`](../../.github/workflows/rust.yml)) with `--require-ipv6`. Any IPv6 case that cannot run then fails instead of being skipped.

## Results recorded so far

| Where | Date | Result |
| --- | --- | --- |
| Agent workspace, Ubuntu 24.04.5, nftables 1.0.9, iproute2 6.1.0, kernel **without IPv6** | 2026-10-15 | 37 passed, 0 failed, 10 skipped: every IPv6 case, **not run** |
| GitHub-hosted runner `ubuntu-24.04` with IPv6 | from the N1-03 pull request on | See the **lab** job of each CI run |

## Known limits

- **Locally generated traffic only** (output hook). Forwarding or router setups are not covered.
- **Routes are lab plumbing**, set by the harness. Septuvo's own route and DNS configuration (for example systemd-resolved, and home-domain DNS in split mode, NP-D2) comes with the real client in M5.
- **Not covered here:**
  - captive-portal mode (Q1);
  - boot-time always-on (Q4);
  - sleep/wake;
  - roaming between real interfaces;
  - updates.

  These belong to M5 and the G2 lab.
- **Rules outliving a crash (NP-E5):** they are kernel state, so they stay in place after the process that applied them exits. Every case here runs after that process has exited. A crashing Septuvo service does not exist yet.
- **Not a real network:** the namespaces share one kernel and use virtual links, so this is not a full real-network test. The owner-hardware VM stage (D-026) repeats it before G2.
