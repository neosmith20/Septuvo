# The Septuvo service `septuvod` and the `septuvo` command (N2-02)

**Status:** development preview, **lab only**.
- `septuvod` turns the kill switch on and off inside a throw-away test network namespace. **It has no tunnel, no encryption and protects nothing outside the lab.**
- The Septuvo Protocol is not implemented before the G1 security review, so the service can never report "connected".

| Crate | What | License |
| --- | --- | --- |
| `crates/septuvo-daemon` | `septuvod`, the privileged service | PolyForm Noncommercial 1.0.0 |
| `crates/septuvo-ipc` | The local control protocol | Apache-2.0 (the Apache-2.0 CLI depends on it; approved in the N2 work order) |
| `crates/septuvo-cli` | `septuvo status`, `septuvo connect`, `septuvo disconnect` | Apache-2.0 |

## Why two programs

Only `septuvod` changes the network: today the firewall, and DNS in N2-03. The `septuvo` command never does. It asks the service, which checks who is asking (ARCHITECTURE: separate privileged daemon and unprivileged UI; identity-and-privacy AD-4).

| Socket | Who may use it | Commands |
| --- | --- | --- |
| `RUNTIME_DIR/status.sock` (mode 0666) | Any local user | `STATUS` |
| `RUNTIME_DIR/control/control.sock` (directory 0750 `root:GROUP`, socket 0660) | Members of the control group | `STATUS`, `CONNECT`, `DISCONNECT` |

**How access is enforced:**
- The kernel enforces access through file permissions.
- The control socket sits inside a directory that only the control group can enter, so it is never reachable by others, not even for the moment between creating the socket and setting its mode.
- The service refuses to start if an existing runtime directory is not a root-owned directory, or if anything other than an old socket is in the way. It never deletes other files.

## Settings (`septuvod.toml`)

```toml
version = 1
[daemon]
mode = "lab"                              # required; the only mode before G1
lab_namespace = "sv-lab"                  # required; see the guard below
runtime_dir = "/run/septuvo"              # default
control_group = "septuvo"                 # default
device_config = "/etc/septuvo/septuvo.toml"   # required; see config.md
```

**Start-up guards.** The service refuses to start unless every one of these holds. Each is tested.

| Guard | Exit status |
| --- | --- |
| It runs as root | 3 |
| `mode = "lab"` | 1 |
| It runs **inside the network namespace named by `lab_namespace`**, checked like `ip netns identify`: its namespace must be the bind mount `/run/netns/NAME`. The host's own namespace is never such a mount, so a real machine's firewall is never touched | 3 |
| The device configuration is valid (N2-01) | 1 |
| The control group exists | 1 |
| The runtime directory is safe | 4 |

## What the commands do

| Command | Effect |
| --- | --- |
| `septuvo status [--runtime-dir DIR]` | Shows the four checks (D-020): adapter, server identity, traffic, DNS, plus the service state and the kill switch. **Server identity is always "failed"** (no peer authentication before G1), so the result is always "Not connected" (exit 3) |
| `septuvo connect [--runtime-dir DIR]` | Installs the N1 kill switch for the configured policy. Protected traffic is blocked, because no VPN path exists; the server endpoints stay reachable |
| `septuvo disconnect [--runtime-dir DIR]` | A deliberate disconnect: removes the Septuvo rules **and restores the original resolver file**. This is the only way they are removed (Q2) |
| `septuvo captive-portal [--minutes N] [--runtime-dir DIR]` | Captive-portal mode (Q1, N3-02): for **at most 5 minutes**, lets web traffic and the network's own DNS out directly so you can sign in to a hotel or airport network. Then it re-blocks by itself. See below |

**DNS (N2-03).** With tunnel DNS (`[dns] resolvers`, see [config.md](config.md)), connect also:
- allows DNS only to those resolvers, only through the tunnel, plus the listed LAN exceptions directly. All other DNS on UDP/TCP 53, DNS over TLS and DNS over QUIC is dropped, in full-tunnel and split mode alike;
- writes the lab namespace's resolver file, `/etc/netns/NAME/resolv.conf` (what `ip netns exec NAME` shows as `/etc/resolv.conf`), with the tunnel resolvers. The original is kept once as `/etc/netns/.septuvo-NAME.resolv.conf.backup`: outside the namespace directory, because `ip netns exec` mounts every file in it. A second connect, or a connect after a crash, never overwrites that backup.

Status reports `dns_policy` and `resolver_file`. The DNS stage itself stays "unknown": a lookup through a real tunnel needs the Septuvo Protocol. The host's `/etc/resolv.conf` is never touched before G1. DNS over HTTPS cannot be blocked by port: see [lab.md](lab.md#dns-over-https-cannot-be-blocked-by-port-stated-not-hidden).

**If the service stops or crashes,** the rules stay in place (fail closed). After a restart it reports them as `blocking-left-from-earlier-run` and leaves them alone.

## Captive-portal mode (N3-02)

Hotel, airport and café networks often show a sign-in page before anything else works. With the kill switch on, that page cannot load. `septuvo captive-portal` opens a short, limited window:

| Opened, outside the tunnel | Still blocked |
| --- | --- |
| Web traffic: TCP 80 and 443, any address | Everything else: QUIC (UDP 443), other TCP and UDP ports, DNS over TLS/QUIC (853) |
| DNS (UDP and TCP 53) to the **network's own resolvers**: the `nameserver` lines of the original resolver file (the backup while connected), at most 4, never loopback or multicast | DNS to any other server; hosts the policy `block`s; `tunnel` prefixes (tunnel only) |

**How the window ends:**
- **The kernel closes the window, not the service.** The kill-switch table always contains three empty nftables sets with timeouts (`captive_web`, `captive_dns4`, `captive_dns6`). Opening the window adds elements that expire after N minutes, and the kernel removes them itself. **A crashed or killed service therefore cannot leave the bypass open.** The lab kills the service mid-window and checks the window still closes on time.
- **5 minutes is the most.** `--minutes` (1–5, default 5) can only shorten it.
- **Asking again restarts the window from now.** It never adds up.
- **Disconnect ends it at once**, and so does a new connect.
- **Nothing ever opens it automatically**, including "always on" (Q4).

**Who and when:**
- Only control-group members (the control socket), and only while connected (the kill switch on and wanted by this service).
- After a service restart, rules left from the earlier run must be reconnected first.

**Status** shows `captive_portal` open or closed. While open, it also shows the time left (or "unknown" if the window was opened before a restart; it still ends on time) and a warning that traffic is leaving outside the tunnel, unprotected.

**Known limits:**
- **Everything on web ports goes out unprotected while the window is open.** That includes DNS over HTTPS, which cannot be told apart from other HTTPS.
- **The resolver file is not changed during the window**, so it still names the tunnel resolver. Tools that ask the network's resolver directly, or a browser pointed at the sign-in page's address, work. Ordinary name lookups through the system resolver do not. Pointing the resolver file at the network's untrusted resolver for the window is a security decision **left to the owner** and not made here.

**The local protocol** is a single text line per request (`SEPTUVO/1 STATUS`, or `SEPTUVO/1 CAPTIVE N` with N a single digit, 1–5), answered with `SEPTUVO/1 OK` or `SEPTUVO/1 ERR code message`, then `key=value` lines and `END`.
- Requests are at most 64 bytes, and responses at most 4096 bytes and 32 fields.
- Only printable ASCII is allowed.
- One request per connection, with a 5-second time-out.
- Anything malformed is refused and never acted on.

## Lab test

```sh
cargo build -p septuvo-daemon -p septuvo-cli --bins
sudo tests/lab/namespaces/daemon.sh target/debug/septuvod target/debug/septuvo
```

The test builds two namespaces, two throw-away users (one in the control group, one not) and a temporary runtime directory, and removes them all afterwards. It checks cases D01–D17:
- the guards (host namespace, wrong mode, wrong namespace);
- that refused starts leave nothing behind;
- status for any user, and connect refused for non-members;
- that the kill switch really blocks traffic while the endpoint stays reachable;
- that status never says connected;
- that hostile socket input changes nothing;
- deliberate disconnect;
- that a crash keeps blocking and a restart reports the old rules.

CI runs it on every pull request.

**Recorded 2026-10-17** on Linux x86_64 (this workspace, Rust 1.97): **17 passed, 0 failed.**

**Captive-portal lab (N3-02):**

```sh
sudo tests/lab/namespaces/captive.sh [--require-ipv6] target/debug/septuvod target/debug/septuvo
```

It uses three namespaces: a device, a "hotel" network with a sign-in page and a resolver, and the far end of a stand-in tunnel. It also creates two throw-away users. It checks cases CP00–CP31:
- **Refusals:** captive-portal mode is refused while not connected and for non-members.
- **Before** the window: the sign-in page, HTTPS and network DNS are blocked.
- **During** the window: TCP 80/443 and network DNS go out directly, IPv4 and IPv6. Other DNS servers, DNS over TLS, QUIC, other ports and policy-blocked hosts stay blocked, and the tunnel is still used.
- **Status** shows the warning and still never says connected.
- **Service killed mid-window:** the window stays the kernel's, and **after the timeout, with no service running, everything is blocked again**.
- **Restarts and repeats:** a restarted service reports a window it did not open; asking again restarts the window and never adds up.
- **Ending:** disconnect ends it at once, and reconnecting starts closed.

It takes about 70 seconds, because one real 1-minute window runs. CI runs it with IPv6 required on every pull request.

**Recorded 2026-10-17** on Linux x86_64 (this workspace, Rust 1.97, no IPv6 in this kernel): **30 passed, 0 failed, 3 skipped** (the IPv6 cases; CI runs them).

## Not in this task

- Server administration: N2-04.
- A tunnel, peer authentication or anything else that needs the Septuvo Protocol: after G1.
- Running as a system service with an installer: done in N3-01 ([install.md](install.md)).
