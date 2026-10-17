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

**DNS (N2-03).** With tunnel DNS (`[dns] resolvers`, see [config.md](config.md)), connect also:
- allows DNS only to those resolvers, only through the tunnel, plus the listed LAN exceptions directly. All other DNS on UDP/TCP 53, DNS over TLS and DNS over QUIC is dropped, in full-tunnel and split mode alike;
- writes the lab namespace's resolver file, `/etc/netns/NAME/resolv.conf` (what `ip netns exec NAME` shows as `/etc/resolv.conf`), with the tunnel resolvers. The original is kept once as `/etc/netns/.septuvo-NAME.resolv.conf.backup`: outside the namespace directory, because `ip netns exec` mounts every file in it. A second connect, or a connect after a crash, never overwrites that backup.

Status reports `dns_policy` and `resolver_file`. The DNS stage itself stays "unknown": a lookup through a real tunnel needs the Septuvo Protocol. The host's `/etc/resolv.conf` is never touched before G1. DNS over HTTPS cannot be blocked by port: see [lab.md](lab.md#dns-over-https-cannot-be-blocked-by-port-stated-not-hidden).

**If the service stops or crashes,** the rules stay in place (fail closed). After a restart it reports them as `blocking-left-from-earlier-run` and leaves them alone.

**The local protocol** is a single text line per request (`SEPTUVO/1 STATUS`), answered with `SEPTUVO/1 OK` or `SEPTUVO/1 ERR code message`, then `key=value` lines and `END`.
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

## Not in this task

- Server administration: N2-04.
- A tunnel, peer authentication or anything else that needs the Septuvo Protocol: after G1.
- Running as a system service with an installer: later milestones.
