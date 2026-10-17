# Installing the Septuvo lab service on Linux (N3-01)

**Status:** development preview, **lab only**.
- The installed service runs inside its own network namespace, `septuvo-lab`, and never changes your computer's own firewall or DNS.
- It has **no tunnel and no encryption**: "connect" turns the kill switch on inside the lab namespace, and status never reports "connected" before the G1 security review.
- Use a test machine or VM.

**Files:** `packaging/linux/`, PolyForm Noncommercial 1.0.0 (D-033).

## Install

```sh
cargo build --release --locked --bins          # or use binaries built in CI
sudo packaging/linux/install.sh --enable       # install, enable and start
sudo usermod -aG septuvo "$USER"                # who may connect (log in again)
septuvo status
septuvo connect                                 # kill switch on, in the lab namespace
septuvo disconnect
```

| Option | Effect |
| --- | --- |
| `--bindir DIR` | Where the binaries are (default `target/release`) |
| `--prefix /usr` | Installation prefix |
| `--enable` | Also enable and start the service |
| `DESTDIR=/path` | Stage everything under `/path`; nothing on this machine changes, and no root is needed |

**Behaviour:**
- **The installer is idempotent:** running it again changes nothing.
- **Existing files in `/etc/septuvo/` are never overwritten.**
- It refuses to run without root unless `DESTDIR` is set.

## What gets installed

| Path | What |
| --- | --- |
| `/usr/libexec/septuvo/septuvod` | The service (N2-02) |
| `/usr/bin/septuvo` | The command-line tool |
| `/usr/sbin/septuvo-server` | Server administration (N2-04) |
| `/usr/lib/systemd/system/septuvo-lab-netns.service` | Creates the `septuvo-lab` namespace (loopback only) and removes it when stopped |
| `/usr/lib/systemd/system/septuvod.service` | Runs the service **inside** that namespace |
| `/usr/lib/sysusers.d/septuvo.conf` | The `septuvo` control group, and the `septuvo-portal` captive-portal helper account (no login, no home; see [captive-portal.md](captive-portal.md)) |
| `/usr/lib/tmpfiles.d/septuvo.conf` | `/etc/netns/septuvo-lab` (the lab namespace's resolver directory) |
| `/etc/septuvo/septuvod.toml`, `septuvo.toml`, `septuvo-server.toml` | Configuration templates ([config.md](config.md), [daemon.md](daemon.md)) |
| `/usr/share/doc/septuvo/` | License and notice |

## Service hardening

On top of the service's own guards (root, lab mode, the named namespace), `septuvod.service`:
- joins the lab namespace (`NetworkNamespacePath=/run/netns/septuvo-lab`);
- keeps only the capabilities it needs (`CAP_NET_ADMIN CAP_CHOWN CAP_FOWNER`);
- sets `NoNewPrivileges`;
- makes the system read-only (`ProtectSystem=strict`) except `/etc/netns` and its runtime directory;
- hides home directories and devices;
- protects the kernel, clock, hostname and control groups;
- allows only Unix and netlink sockets;
- forbids new namespaces, real-time scheduling, SUID and writable executable memory;
- uses the `@system-service` system-call filter.

**`systemd-analyze security` rates its exposure 2.3 ("OK")**, measured offline on 2026-10-17.

**Stopping the service keeps the kill switch on** (Q2). Only `septuvo disconnect` lifts it.

## Uninstall

```sh
sudo packaging/linux/uninstall.sh            # keeps /etc/septuvo and server device records
sudo packaging/linux/uninstall.sh --purge    # removes them too
```

The uninstaller runs in this order:
1. A deliberate disconnect, which removes the kill switch and restores the lab namespace's resolver file.
2. It stops and removes the units.
3. It removes leftover rules, even if the service had crashed.
4. It removes the lab namespace and every installed file.
5. It removes the `septuvo-portal` helper account and the `septuvo` group.

**No Septuvo rules are left behind.** It says what it kept.

## Tests

```sh
tests/lab/install/run.sh target/release              # staged install (any machine)
sudo tests/lab/install/run.sh --real target/release  # DISPOSABLE MACHINES ONLY
```

- **Staged (I01–I06):**
  - the exact file list and modes (`tests/lab/install/expected-files.txt`);
  - the unit points at the binary;
  - an existing config is never overwritten;
  - a second install changes nothing;
  - it refuses without root.
- **Real (I10–I22),** run by CI on a disposable runner:
  - install;
  - `systemd-analyze verify`;
  - the service runs in the lab namespace;
  - the captive-portal helper account exists (never root), and the installed rules are scoped to it;
  - status for any user, and connect refused for non-members;
  - the kill switch on in the lab namespace, with **the host firewall untouched**;
  - stopping keeps blocking, and disconnect after a restart;
  - re-install;
  - **uninstall while connected leaves nothing** (files, units, group, helper account, namespace, rules);
  - config kept, then `--purge`.

**Recorded 2026-10-17** in this workspace (Linux x86_64, Rust 1.97; it has no systemd as its init system, so only the staged part can run here): **5 passed, 0 failed.** `systemd-analyze verify` passes for both units, run offline. The real install runs in CI.

## Not in this task

- Distribution packages (`.deb`, `.rpm`) and signed releases: G3.
- Running outside the lab namespace on a real network: a separate work order and risk review.
- Other operating systems.
