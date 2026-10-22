# Septuvo configuration files (N2-01, N4-03)

**Status:** development preview. These files configure parts of Septuvo that exist today: routing policy, DNS policy and local settings on a device, and address ranges and storage on the server. **They do not make a working VPN.** The Septuvo Protocol and enrollment are not implemented before the G1 security review, so nothing here connects to anything.

**Code:** `crates/septuvo-config`, PolyForm Noncommercial 1.0.0 (D-033).

## Checking a file

```sh
cargo run -p septuvo-config -- check device crates/septuvo-config/examples/device-full-tunnel.toml
cargo run -p septuvo-config -- check server crates/septuvo-config/examples/server.toml
```

| Exit status | Meaning |
| --- | --- |
| 0 | The file is valid; a summary is printed |
| 1 | Invalid, with line, column and setting named |
| 2 | Usage error |

Nothing is applied.

## The format: a strict subset of TOML

Every file Septuvo accepts is valid [TOML](https://toml.io/en/v1.0.0), so standard editors and tools understand it. Septuvo accepts only a small part of TOML, because configuration is treated as hostile input.

| Supported | Not supported (clear error) |
| --- | --- |
| `[table]` headers | dotted or quoted names, `[[arrays of tables]]` |
| `key = value` | inline tables `{ ... }` |
| strings `"..."` (with escapes) and `'...'` | multi-line strings |
| decimal integers | floats, dates, hex/octal/binary, `_` separators |
| `true` / `false` | |
| arrays of strings, which may span lines, with comments and a trailing comma | arrays of other types |
| `#` comments, LF or CRLF line endings | |

**Limits:**
- 64 KiB per file;
- 1024 characters per line;
- 512 characters per string;
- 256 array items;
- 64 tables;
- 128 keys per table.

**Strictness:**
- A key or table defined twice is an error.
- Unknown keys and tables are errors, so a typo never silently does nothing.
- Every file starts with `version = 1`.

## Device file (`septuvo.toml`)

| Setting | Required | Meaning |
| --- | --- | --- |
| `version` | yes | Must be `1` |
| `server.endpoints` | yes | 1 to 16 entries such as `"203.0.113.10:51820"` or `"[2001:db8::10]:51820"`. IP addresses only: no host names (no DNS outside the tunnel is needed to connect, NP-U3). Not loopback, multicast or unspecified; port 1–65535; no duplicates. An endpoint may not lie inside a `tunnel` or `block` rule |
| `server.identity` | — | **Refused before G1.** Server identities come from enrollment, which does not exist yet. The summary shows "not enrolled" |
| `routing.mode` | yes | `"full-tunnel"` (journey B) or `"split"` (journey A, D-027). In split mode, traffic outside the listed networks is **not** protected |
| `routing.tunnel` / `direct` / `block` | no | Prefix lists, for example `"192.168.1.30/32"`. Host bits must be zero. Same rules as the N1 policy model: most specific prefix wins, duplicates are rejected, and a `direct` rule for `0.0.0.0/0` or `::/0` is rejected because it would switch protection off |
| `[dns]` | yes | Must be present |
| `dns.mode` | no | `"tunnel"`, the default and the only value accepted. **`"system"` is refused in both modes (N4-03, NP-D10):** in full-tunnel mode it would send DNS outside the tunnel; in split mode it stays refused until names in the protected networks are always resolved through the tunnel (per-domain DNS, NP-D2) |
| `dns.resolvers` | yes, in tunnel mode | 1 to 4 resolver addresses. Each must be routed **through the tunnel** by `[routing]`; otherwise the file is rejected. Loopback resolvers are refused. In tunnel mode, all other DNS (UDP/TCP 53, DNS over TLS, DNS over QUIC) is blocked, in full-tunnel and split mode alike, and the resolver file points at these resolvers while connected ([daemon.md](daemon.md)). DNS over HTTPS cannot be blocked by port ([lab.md](lab.md#dns-over-https-cannot-be-blocked-by-port-stated-not-hidden)) |
| `dns.lan_exceptions` | no | Up to 8 local resolvers allowed to receive DNS directly. Each must also be covered by a `routing.direct` rule |
| `tunnel.interface` | no | 1–15 characters from `A-Z a-z 0-9 _ - .`; default `septuvo0` |
| `tunnel.mtu` | no | 1280–9000; default 1420 (wire-format.md §5.2) |
| `[behavior]` | — | **Refused since N4-03.** `behavior.always_on` was retired by D-038: it conflated two things that are now separate settings, **Lockdown Mode** (a service setting, blocking without the VPN) and **Connect Automatically at Startup** (an app setting). The error says so; remove the table |
| `connection.name` | no | The friendly name of a **saved connection** (N4-03): 1–64 characters after trimming, no control characters, any language. Required when the file is saved with `septuvo connection add` (or give `--name`). The service keeps the name separately and adds it back on export, so exported files import again under the same name |
| `logging.retention_days` | no | 0–365; default 7 (Q10) |
| `[kill_switch]` | — | **Refused.** The kill switch is mandatory (D-038) and is lifted only by a deliberate disconnect (Q2), and not even then while Lockdown Mode is on (planned, N4-04) |

## Server file (`septuvo-server.toml`)

| Setting | Required | Meaning |
| --- | --- | --- |
| `version` | yes | Must be `1` |
| `listen.address` | no | IP address; default `::`. **Stored only: no port is opened before G1** |
| `listen.port` | yes | 1–65535 |
| `network.ipv4_range` | yes | Device tunnel addresses. Must lie inside 10.0.0.0/8, 172.16.0.0/12 or 192.168.0.0/16, with length /16 to /30 (Q13) |
| `network.ipv6_range` | yes | Inside the unique-local range fc00::/7, with length /48 to /120 |
| `storage.data_dir` | no | An absolute path without `.`, `..`, `//` or a trailing slash; default `/var/lib/septuvo-server` |
| `logging.retention_days` | no | 0–365; default 7 |

## Examples

`crates/septuvo-config/examples/`. A unit test keeps them valid.

| File | What it shows |
| --- | --- |
| `device-full-tunnel.toml` | Untrusted Wi-Fi: everything through the tunnel, one direct printer/NAS |
| `device-split.toml` | Home network from anywhere: only the home networks use the tunnel |
| `server.toml` | A server with private tunnel ranges |

## Error messages

Errors name the position, the setting and the reason, for example:

```text
septuvo.toml: line 7, column 8: dns.mode: "system" DNS would send queries outside the tunnel in full-tunnel mode
```

## Saved connections (N4-03)

The service keeps the user's **complete** connections (N4-D11): each is a device file as above, validated before it is stored. Drafts stay in the app. See [daemon.md](daemon.md#saved-connections-n4-03).

## Changes still to come from the approved desktop UI

Global DNS settings with per-connection overrides (Settings → DNS Protection, N4-07) and the service settings for Lockdown Mode and Local Network Access (N4-04). Details: [desktop-ui-reconciliation.md](../reviews/desktop-ui-reconciliation.md).

## Not in this task

- Applying the configuration: the privileged service is N2-02.
- Anything that needs enrollment or the Septuvo Protocol (after G1).
