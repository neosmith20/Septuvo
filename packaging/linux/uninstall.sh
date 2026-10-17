#!/bin/sh
# Septuvo VPN uninstaller for Linux (development preview).
#
#   sudo packaging/linux/uninstall.sh [--prefix /usr] [--purge]
#
# Disconnects first (removing the kill switch and restoring the lab
# namespace's resolver file), then removes the units, the lab namespace,
# every installed file and the "septuvo" group. No Septuvo rules are left
# behind. /etc/septuvo and the server's device records in
# /var/lib/septuvo-server are kept unless --purge is given.
# Licensed under PolyForm Noncommercial 1.0.0 (see NOTICE).
set -u

PREFIX=/usr
PURGE=0
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) PREFIX=${2:?--prefix needs a directory}; shift 2 ;;
    --purge) PURGE=1; shift ;;
    -h|--help) sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "uninstall.sh: unknown option $1" >&2; exit 2 ;;
  esac
done
if [ "$(id -u)" != 0 ]; then
  echo "uninstall.sh: needs root" >&2
  exit 2
fi
NS=septuvo-lab
say() { echo "uninstall: $*"; }

# 1. Deliberate disconnect while the service still runs (Q2).
if [ -x "$PREFIX/bin/septuvo" ] && [ -S /run/septuvo/control/control.sock ]; then
  "$PREFIX/bin/septuvo" disconnect >/dev/null 2>&1 && say "disconnected"
fi
# 2. Stop and remove the units.
for unit in septuvod.service septuvo-lab-netns.service; do
  systemctl disable --now "$unit" >/dev/null 2>&1 || true
done
# 3. Remove any rules and the resolver file even if the service was dead.
if ip netns list 2>/dev/null | grep -qw "$NS"; then
  ip netns exec "$NS" nft delete table inet septuvo 2>/dev/null && say "removed leftover kill-switch rules"
fi
BACKUP="/etc/netns/.septuvo-$NS.resolv.conf.backup"
RESOLV="/etc/netns/$NS/resolv.conf"
if [ -e "$BACKUP" ]; then
  mv -f "$BACKUP" "$RESOLV" && say "restored the lab namespace's original resolver file"
elif [ -f "$RESOLV" ] && head -1 "$RESOLV" | grep -q "^# Written by septuvod"; then
  rm -f "$RESOLV"
fi
rm -f "/etc/netns/.septuvo-$NS.resolv.conf.tmp"
rmdir "/etc/netns/$NS" 2>/dev/null || true
rmdir /etc/netns 2>/dev/null || true
if ip netns list 2>/dev/null | grep -qw "$NS"; then
  ip netns delete "$NS" && say "removed the lab namespace"
fi
# 4. Remove installed files.
for f in \
  "$PREFIX/libexec/septuvo/septuvod" "$PREFIX/bin/septuvo" "$PREFIX/sbin/septuvo-server" \
  "$PREFIX/lib/systemd/system/septuvod.service" "$PREFIX/lib/systemd/system/septuvo-lab-netns.service" \
  "$PREFIX/lib/sysusers.d/septuvo.conf" "$PREFIX/lib/tmpfiles.d/septuvo.conf" \
  "$PREFIX/share/doc/septuvo/LICENSE.md" "$PREFIX/share/doc/septuvo/NOTICE"; do
  rm -f "$f"
done
rmdir "$PREFIX/libexec/septuvo" "$PREFIX/share/doc/septuvo" 2>/dev/null || true
rm -rf /run/septuvo
systemctl daemon-reload 2>/dev/null || true
# 5. The control group.
if getent group septuvo >/dev/null; then
  groupdel septuvo && say "removed group septuvo"
fi
if [ "$PURGE" = 1 ]; then
  rm -rf /etc/septuvo /var/lib/septuvo-server
  say "purged /etc/septuvo and /var/lib/septuvo-server"
else
  [ -d /etc/septuvo ] && say "kept /etc/septuvo (use --purge to remove it)"
  [ -d /var/lib/septuvo-server ] && say "kept /var/lib/septuvo-server device records (use --purge to remove them)"
fi
say "done"
