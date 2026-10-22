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
device_config = "/etc/septuvo/septuvo.toml"   # required; used when no saved connection is selected
connections_dir = "/var/lib/septuvo/connections"   # default; saved connections (N4-03)
```

**Start-up guards.** The service refuses to start unless every one of these holds. Each is tested.

| Guard | Exit status |
| --- | --- |
| It runs as root | 3 |
| `mode = "lab"` | 1 |
| It runs **inside the network namespace named by `lab_namespace`**, checked like `ip netns identify`: its namespace must be the bind mount `/run/netns/NAME`. The host's own namespace is never such a mount, so a real machine's firewall is never touched | 3 |
| The device configuration is valid (N2-01) | 1 |
| The connection store is a private root-owned directory (created mode 0700 if missing) and its index is intact: **a damaged store is never guessed at** | 1 |
| The control group exists | 1 |
| The runtime directory is safe | 4 |

## What the commands do

| Command | Effect |
| --- | --- |
| `septuvo status [--runtime-dir DIR]` | Shows the four checks (D-020): adapter, server identity, traffic, DNS, plus the service state and the kill switch. **Server identity is always "failed"** (no peer authentication before G1), so the result is always "Not connected" (exit 3) |
| `septuvo connect [--runtime-dir DIR]` | Installs the N1 kill switch for the policy of the **selected connection** (or the settings' `device_config` if none is selected). Protected traffic is blocked, because no VPN path exists; the server endpoints stay reachable |
| `septuvo connect CONN` | Selects a saved connection and connects (N4-03) |
| `septuvo connection …` | Saved connections: `list`, `show`, `export`, `add`, `replace`, `rename`, `delete`, `select`. See below |
| `septuvo disconnect [--runtime-dir DIR]` | A deliberate disconnect: removes the Septuvo rules **and restores the original resolver file**. This is the only way they are removed (Q2) |
| `septuvo captive-portal [--minutes N] [--network-dns] [--runtime-dir DIR]` | Captive-portal mode (Q1, N3-02): for **at most 5 minutes**, lets **only the sign-in helper account** reach web servers outside the tunnel. Every other program stays blocked. It then re-blocks by itself. See below |
| `septuvo watch [--interval S] [--count N] [--runtime-dir DIR]` | The live status view (N3-04), redrawn every S seconds (1–60, default 2) until Ctrl-C, or for N refreshes. Read-only, over the status socket, so any user may run it. See below |

**DNS (N2-03).** With tunnel DNS (`[dns] resolvers`, see [config.md](config.md)), connect also:
- allows DNS only to those resolvers, only through the tunnel, plus the listed LAN exceptions directly. All other DNS on UDP/TCP 53, DNS over TLS and DNS over QUIC is dropped, in full-tunnel and split mode alike;
- writes the lab namespace's resolver file, `/etc/netns/NAME/resolv.conf` (what `ip netns exec NAME` shows as `/etc/resolv.conf`), with the tunnel resolvers. The original is kept once as `/etc/netns/.septuvo-NAME.resolv.conf.backup`: outside the namespace directory, because `ip netns exec` mounts every file in it. A second connect, or a connect after a crash, never overwrites that backup.

Status reports `dns_policy` and `resolver_file`. The DNS stage itself stays "unknown": a lookup through a real tunnel needs the Septuvo Protocol. The host's `/etc/resolv.conf` is never touched before G1. DNS over HTTPS cannot be blocked by port: see [lab.md](lab.md#dns-over-https-cannot-be-blocked-by-port-stated-not-hidden).

**If the service stops or crashes,** the rules stay in place (fail closed). After a restart it reports them as `blocking-left-from-earlier-run` and leaves them alone.

## Saved connections (N4-03)

The service keeps the user's complete connections (N4-D11) in `connections_dir`, a private root-owned directory:

```text
index.toml      order, names, the selected connection and the next ID
c0001.toml      a connection's configuration (a device file, without its name)
```

- **Only valid, complete connections are stored.** Each is checked by `septuvo-config` first; a bad one is refused and **changes nothing**. Every write is atomic (temporary file, `fsync`, rename).
- **IDs** (`c0001`–`c9999`) are assigned by the service and never reused. **Names** are 1–64 characters, unique regardless of case. At most 32 connections.
- **Rename** changes only the name: no network change. **Export** returns the configuration with a `[connection] name` table, so it imports again under the same name. **Delete** refuses the connection in use while connected; deleting the selected one selects the next.
- **List** is open to every local user (names only); everything else needs the control group.

```sh
septuvo connection add home.toml --name "Home VPN"   # Saved as c0001 (not selected)
septuvo connection list                              # * marks the selected one
septuvo connect "Home VPN"                           # select and connect
septuvo connection select Office                     # while connected: switch
septuvo connection export c0001 home-copy.toml       # never overwrites a file
```

**Switching while connected (D-041).** Selecting another connection while connected replaces the whole kill-switch ruleset **in one nftables transaction**: `nft -f` applies the script atomically (the old table is deleted and the new one created together), so there is no moment without protection. The tunnel resolvers are then pointed at the new connection's. **If the switch fails,** the kernel keeps the previous rules (still blocking), the new selection is kept, status reports `state=failed` with the reason in `failure`, and the old connection is **never silently resumed**; `connect` retries.

**Protocol.** Requests that carry a configuration or a name (`ADD`, `REPLACE ID`, `RENAME ID`) and responses that return one (`GET ID`, `LIST`) have a **length-prefixed body of at most 64 KiB**, the one exception to the 64-byte request limit. The length comes first, so the service never reads more than announced; one request per connection, and anything after it is never read. Specified in `crates/septuvo-ipc` and fuzzed (`fuzz_ipc_framed`, [fuzzing.md](fuzzing.md)).

**New status fields** (for the desktop app's tiles):

| Field | Values |
| --- | --- |
| `state` | `disconnected`, `blocking`, `blocking-left-from-earlier-run`, `error-kill-switch-missing`, and now `failed` (with `failure`). `connecting`, `switching`, `disconnecting` and `reconnecting` are reserved for operations that take time; before G1 every operation is one atomic firewall transaction done under the service's lock, so status never observes them |
| `failure` | Why the last connect or switch failed, until a success or a deliberate disconnect |
| `connection` | The connection in use: its ID, or `default` (the settings' `device_config`) |
| `dns_protection` | `on` while the ruleset is installed (DNS leak prevention is part of the same atomic ruleset as the kill switch, so it is verified the same way), `failed` if it should be but is not, `off` otherwise |
| `verified_at` | When this status was checked (Unix seconds) |

**Configuration changes** (N4-03): `[behavior]` (`always_on`) is refused (D-038), and `dns.mode = "system"` is refused in split mode too (NP-D10). See [config.md](config.md).

## Live status view (N3-04)

`septuvo watch` shows what `septuvo status` shows, refreshed in place:
- the four checks;
- the service state and the kill switch;
- the DNS policy and the resolver file;
- the captive-portal window, with its countdown and warning.

**How it stays honest:**
- **Every refresh is a fresh answer.** Each frame is drawn only from the status the service gave for that refresh; nothing is remembered.
- **A lost service is shown at once.** If the service stops, crashes or cannot be reached, the next frame says **SERVICE LOST** and shows every check as unknown, never the last known state.
- **"Connected" only when all four checks pass**, through the same status model as `septuvo status` (D-020). Before G1 that never happens.

**Output:**
- **On a terminal:** redrawn in place, with colour (red for a lost service and warnings, yellow for "not connected"); `NO_COLOR=1` turns colour off.
- **In a file or pipe:** frames are printed one after another as plain text with a `----` separator, with no escape sequences.

**Standard library only.** A desktop or tray app would use the same status interface; a GUI toolkit is a separate dependency decision (D-015).

## Captive-portal mode (N3-02)

`septuvo captive-portal [--minutes N] [--network-dns]` opens a sign-in window of at most 5 minutes. It applies **only to the dedicated helper account** (`daemon.captive_helper_user`, created by the packages as `septuvo-portal`, never root); every other program stays fully blocked. Further limits:
- **Web only, outside the tunnel:** the helper may use TCP 80/443 outside the tunnel, and nothing else.
- **DNS:** encrypted DNS over HTTPS by default. The network's own DNS only with `--network-dns`, for the helper, for that window. The resolver file is never changed.
- **Expiry:** the kernel ends the window even if the service dies.
- **Limits:** at most 3 windows per connection, each logged.

Design, the RFC 8910/8908 and DoH evaluation, unsupported legacy portals and the phased plan: **[captive-portal.md](captive-portal.md)**.

**The local protocol** is a single text line per request (`SEPTUVO/1 STATUS`, or `SEPTUVO/1 CAPTIVE N [NETWORK-DNS]` with N a single digit, 1–5), answered with `SEPTUVO/1 OK` or `SEPTUVO/1 ERR code message`, then `key=value` lines and `END`.
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

**Live status view lab (N3-04):**

```sh
sudo tests/lab/namespaces/watch.sh target/debug/septuvod target/debug/septuvo
```

`septuvo watch` runs as an ordinary user, outside the control group, refreshing every second. Meanwhile the lab connects, opens a 1-minute captive-portal window, disconnects, kills the service and restarts it. Cases W00–W11 check that:
- the output followed each change;
- the countdown counted down;
- **a killed service showed SERVICE LOST at once, with no last known state**;
- no frame ever claimed "Connected";
- the output was plain text.

**Recorded 2026-10-17** in this workspace (Linux x86_64, Rust 1.97): **12 passed, 0 failed**, three runs in a row. CI runs it on every pull request.

**Saved-connections and switching lab (N4-03, T2-SW-01):**

```sh
sudo tests/lab/namespaces/switch.sh target/debug/septuvod target/debug/septuvo
```

Connection A allows only UDP 5000 to the network side, B only UDP 443; port 53 is outside both. While a sender floods all three ports, the service switches A↔B 20 times and a watcher checks the ruleset continuously. Cases S00–S20 and T2-SW-01a–c check that:
- **no packet outside both policies got out** (port 53: 0 arrived, about 370,000 refused);
- each connection's own port got through while it was in use;
- **the rules were never missing**;
- bad connections (bad resolver, system DNS, `always_on`) are refused and change nothing, including for the connection in use;
- names (Unicode, unique regardless of case), list for everyone, configurations and changes for the control group only;
- the selection survives a service restart; export, rename (no rule change), delete and re-import.

**Negative control:** the same lab with a non-atomic switch (disconnect, select, connect) **fails** T2-SW-01a (52,413 port-53 packets got out) and T2-SW-01c (94 checks without rules), so the lab does detect a gap.

**Recorded 2026-10-22** in this workspace (Linux x86_64, Rust 1.97): **24 passed, 0 failed.** CI runs it on every pull request.

**Captive-portal lab (N3-02):** `sudo tests/lab/namespaces/captive.sh [--require-ipv6] target/debug/septuvod target/debug/septuvo`, cases CP00–CP53. What it proves: [captive-portal.md](captive-portal.md#tests).

## Planned changes from the approved desktop UI (not implemented)

The owner's desktop decisions (D-037–D-042) still need, in the service (saved connections and switching are done, N4-03):
- **Lockdown Mode**: blocking after a deliberate disconnect, across restarts and from boot. Today a deliberate disconnect always removes the rules;
- **Local Network Access** exceptions for the directly attached network;
- DNS settings (N4-07);
- `DIAGNOSE` and pings on request.

They are scheduled in the revised N4 work order. Nothing in this document claims them before they exist. Details: [desktop-ui-reconciliation.md](../reviews/desktop-ui-reconciliation.md).

## Not in this task

- Server administration: N2-04.
- A tunnel, peer authentication or anything else that needs the Septuvo Protocol: after G1.
- Running as a system service with an installer: done in N3-01 ([install.md](install.md)).
