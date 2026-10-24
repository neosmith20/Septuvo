#!/bin/sh
# Septuvo VPN installer for Linux (development preview, lab only).
#
#   sudo packaging/linux/install.sh [--bindir DIR] [--prefix /usr] [--enable]
#   DESTDIR=/tmp/stage packaging/linux/install.sh [--bindir DIR]
#
# Installs prebuilt binaries (default: target/release), hardened systemd units,
# the "septuvo" control group and configuration templates. Existing files in
# /etc/septuvo are never overwritten. Running it again changes nothing.
# With DESTDIR set, everything goes into that staging directory and nothing on
# this machine is changed. --enable also enables and starts the service.
#
# Before the G1 security review the service runs only inside the
# "septuvo-lab" network namespace: it has no tunnel and protects nothing
# outside the lab. Licensed under PolyForm Noncommercial 1.0.0 (see NOTICE).
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
BINDIR="$HERE/../../target/release"
PREFIX=/usr
ENABLE=0
while [ $# -gt 0 ]; do
  case "$1" in
    --bindir) BINDIR=${2:?--bindir needs a directory}; shift 2 ;;
    --prefix) PREFIX=${2:?--prefix needs a directory}; shift 2 ;;
    --enable) ENABLE=1; shift ;;
    -h|--help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "install.sh: unknown option $1" >&2; exit 2 ;;
  esac
done
case "$PREFIX" in
  /*) ;;
  *) echo "install.sh: --prefix must be an absolute path" >&2; exit 2 ;;
esac
DEST=${DESTDIR:-}
if [ -z "$DEST" ] && [ "$(id -u)" != 0 ]; then
  echo "install.sh: needs root (or set DESTDIR to stage the files)" >&2
  exit 2
fi
if [ -n "$DEST" ] && [ "$ENABLE" = 1 ]; then
  echo "install.sh: --enable cannot be used with DESTDIR" >&2
  exit 2
fi
for bin in septuvod septuvo septuvo-server; do
  [ -x "$BINDIR/$bin" ] || { echo "install.sh: missing $BINDIR/$bin (build with: cargo build --release --locked --bins)" >&2; exit 2; }
done

LIBEXECDIR="$PREFIX/libexec/septuvo"
UNITDIR="$PREFIX/lib/systemd/system"
say() { echo "install: $*"; }

install -d -m 755 "$DEST$LIBEXECDIR" "$DEST$PREFIX/bin" "$DEST$PREFIX/sbin" "$DEST$UNITDIR" \
  "$DEST$PREFIX/lib/sysusers.d" "$DEST$PREFIX/lib/tmpfiles.d" "$DEST$PREFIX/share/doc/septuvo" \
  "$DEST/etc/septuvo"
install -m 755 "$BINDIR/septuvod" "$DEST$LIBEXECDIR/septuvod"
install -m 755 "$BINDIR/septuvo" "$DEST$PREFIX/bin/septuvo"
install -m 755 "$BINDIR/septuvo-server" "$DEST$PREFIX/sbin/septuvo-server"
for unit in septuvod.service septuvo-lockdown.service; do
  sed "s|@LIBEXECDIR@|$LIBEXECDIR|g" "$HERE/$unit" > "$DEST$UNITDIR/$unit.tmp"
  chmod 644 "$DEST$UNITDIR/$unit.tmp"
  mv "$DEST$UNITDIR/$unit.tmp" "$DEST$UNITDIR/$unit"
done
install -m 644 "$HERE/septuvo-lab-netns.service" "$DEST$UNITDIR/septuvo-lab-netns.service"
install -m 644 "$HERE/septuvo.sysusers" "$DEST$PREFIX/lib/sysusers.d/septuvo.conf"
install -m 644 "$HERE/septuvo.tmpfiles" "$DEST$PREFIX/lib/tmpfiles.d/septuvo.conf"
install -m 644 "$HERE/LICENSE.md" "$DEST$PREFIX/share/doc/septuvo/LICENSE.md"
install -m 644 "$HERE/NOTICE" "$DEST$PREFIX/share/doc/septuvo/NOTICE"
for conf in septuvod.toml septuvo.toml septuvo-server.toml; do
  if [ -e "$DEST/etc/septuvo/$conf" ]; then
    say "kept existing /etc/septuvo/$conf"
  else
    install -m 644 "$HERE/config/$conf" "$DEST/etc/septuvo/$conf"
  fi
done
say "files installed${DEST:+ under $DEST}"

if [ -z "$DEST" ]; then
  systemd-sysusers "$PREFIX/lib/sysusers.d/septuvo.conf"
  systemd-tmpfiles --create "$PREFIX/lib/tmpfiles.d/septuvo.conf"
  systemctl daemon-reload
  say "group 'septuvo' ready: add users who may connect with: usermod -aG septuvo USER"
  if [ "$ENABLE" = 1 ]; then
    systemctl enable --now septuvo-lab-netns.service septuvo-lockdown.service septuvod.service
    say "service enabled and started (lab namespace septuvo-lab)"
  else
    systemctl enable septuvo-lockdown.service
    say "start it with: systemctl enable --now septuvod.service"
  fi
  say "Lockdown Mode is off by default; when turned on it also applies from boot"
fi
say "development preview: no tunnel, no encryption; status never reports connected before the G1 review"
