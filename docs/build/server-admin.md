# Septuvo Server administration: `septuvo-server` (N2-04)

**Status:** development preview. `septuvo-server` keeps the server's **device records**: which devices are enrolled, what each may reach, and its tunnel addresses.

**No device can connect yet.**
- There are no invitations, no enrollment and no keys.
- No network port is opened.
- All of that needs the Septuvo Protocol and the G1 security review.

Each record holds a **placeholder key** that is not a key and authenticates nothing. Today's records and commands are what reviewed enrollment will fill in later.

**Code:** `crates/septuvo-server`, PolyForm Noncommercial 1.0.0 (D-033). Runs locally on the server host; there is no network administration interface (AD-1).

## Commands

```sh
septuvo-server --config /etc/septuvo/septuvo-server.toml config check
septuvo-server --config FILE device add NAME --scope home|exit|both
septuvo-server --config FILE device list
septuvo-server --config FILE device show NAME
septuvo-server --config FILE device revoke NAME
septuvo-server --config FILE device remove NAME     # revoked devices only
```

| Rule | Why |
| --- | --- |
| Names are 1–32 characters from `a-z 0-9 - _ .`, starting with a letter or digit | Names reach logs and terminals; nothing else is accepted, and nothing hostile is ever echoed |
| Addresses: the lowest free ones in the configured ranges (N2-01, Q13). The network address, the server's own address (offset 1) and the IPv4 broadcast address are never given out | No collisions; predictable |
| A revoked device keeps its addresses until it is **removed** | An address is never reused while an old device might still claim it |
| Only revoked devices can be removed | A device is never deleted while it is still allowed |
| `device add` always says the device cannot connect yet | No false impression of a working VPN |

Exit status: 0 success, 1 error, 2 usage error.

## Storage (`storage.data_dir`, default `/var/lib/septuvo-server`)

| File | Mode | Content |
| --- | --- | --- |
| `devices.db` | 0600 | The records, one line per device, in a strict text format |
| `devices.lock` | 0600 | Held while a command runs; a second command at the same time is refused |
| `actions.log` | 0600 | `time action name` per change (add, revoke, remove). No secrets (AD-2, PR-2) |

The directory itself is 0700.

**Crash safety.** Records are written to a temporary file, flushed to disk, renamed over the old file, and then the directory is flushed. An interruption leaves the old or the new file, never a half-written one. A unit test interrupts a save before the rename and checks that the old records are intact.

**Fail closed (RV-2).**
- A missing records file means "no devices yet".
- A file that exists but is damaged, has an unknown format version, contains a real-looking key, or has addresses outside the configured ranges makes **every** command refuse, with the line and the reason. It is never treated as empty, so a revoked device can never come back by damaging the file.
- Symbolic links are refused.

## Tests

`cargo test -p septuvo-server` covers:
- allocation, including a range running out;
- revoke, remove and reserved addresses;
- hostile names;
- the round trip of the format, and 15 kinds of damaged file;
- private permissions;
- an interrupted save;
- the lock;
- symbolic links;
- the full command lifecycle with the action log;
- usage errors, including `device invite`, which does not exist before G1.

## Not in this task

- Invitations and enrollment (EN-1 to EN-9): after G1.
- Server keys, the network listener and session handling: after G1.
- Log rotation by `logging.retention_days`: later work.
