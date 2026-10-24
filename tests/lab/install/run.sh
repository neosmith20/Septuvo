#!/usr/bin/env bash
# Septuvo install lab (N3-01).
#
#   tests/lab/install/run.sh BINDIR            staged install only (any machine)
#   sudo tests/lab/install/run.sh --real BINDIR  also a real install, use and
#                                              uninstall: DISPOSABLE MACHINES ONLY
#                                              (CI runner, owner lab VM)
#
# BINDIR holds release binaries (cargo build --release --locked --bins).
# Part A installs into a staging directory and checks the exact file list,
# modes and idempotency. Part B (--real) installs onto this machine with
# systemd, uses the service, then uninstalls and checks nothing is left.
set -euo pipefail

REAL=0
if [ "${1:-}" = "--real" ]; then REAL=1; shift; fi
BINDIR=$(realpath "${1:?usage: run.sh [--real] BINDIR}")
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
PKG="$ROOT/packaging/linux"
EXPECTED="$(dirname "$0")/expected-files.txt"
WORK=$(mktemp -d)
PASS=0 FAIL=0 N=0
result() {
  N=$((N + 1))
  if [ "$1" = 1 ]; then PASS=$((PASS + 1)); echo "PASS  $2"
  else FAIL=$((FAIL + 1)); echo "FAIL  $2"; [ -n "${GITHUB_ACTIONS:-}" ] && echo "::error title=install lab::$2"; fi
}
# Prints the details of a failure (a file, or standard input), indented; in
# CI also as annotations, since the job log is not always at hand.
detail() {
  local n=0
  while IFS= read -r line; do
    echo "      $line"
    n=$((n + 1))
    if [ -n "${GITHUB_ACTIONS:-}" ] && [ $n -le 8 ]; then echo "::warning title=install lab detail::$line"; fi
  done < "${1:-/dev/stdin}"
}
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

# Lists "mode path" for every file and directory under a staging root.
listing() { (cd "$1" && find . -mindepth 1 -printf '%m %y %P\n' | LC_ALL=C sort); }

# --- Part A: staged install ----------------------------------------------------
set +e
STAGE="$WORK/stage"
DESTDIR="$STAGE" "$PKG/install.sh" --bindir "$BINDIR" > "$WORK/a1.log" 2>&1; rc=$?
[ $rc = 0 ] && ok=1 || ok=0
result $ok "I01 staged install succeeds without root changes (exit $rc)"
listing "$STAGE" > "$WORK/got.txt"
diff -u "$EXPECTED" "$WORK/got.txt" > "$WORK/diff.txt" && ok=1 || ok=0
result $ok "I02 staged file list and modes are exactly as expected"
[ $ok = 1 ] || cat "$WORK/diff.txt"
grep -q "^ExecStart=/usr/libexec/septuvo/septuvod --settings /etc/septuvo/septuvod.toml$" "$STAGE/usr/lib/systemd/system/septuvod.service" && ok=1 || ok=0
result $ok "I03 the service unit points at the installed binary"
(cd "$STAGE" && find . -type f -exec sha256sum {} + | LC_ALL=C sort) > "$WORK/sum1.txt"
echo "# local edit" >> "$STAGE/etc/septuvo/septuvo.toml"
DESTDIR="$STAGE" "$PKG/install.sh" --bindir "$BINDIR" > "$WORK/a2.log" 2>&1
grep -q "# local edit" "$STAGE/etc/septuvo/septuvo.toml" && grep -q "kept existing /etc/septuvo/septuvo.toml" "$WORK/a2.log" && ok=1 || ok=0
result $ok "I04 re-installing never overwrites an existing configuration"
sed -i '$d' "$STAGE/etc/septuvo/septuvo.toml"
DESTDIR="$STAGE" "$PKG/install.sh" --bindir "$BINDIR" > /dev/null 2>&1
(cd "$STAGE" && find . -type f -exec sha256sum {} + | LC_ALL=C sort) > "$WORK/sum2.txt"
cmp -s "$WORK/sum1.txt" "$WORK/sum2.txt" && ok=1 || ok=0
result $ok "I05 re-installing changes nothing (idempotent)"
if [ "$(id -u)" != 0 ]; then
  "$PKG/install.sh" --bindir "$BINDIR" > /dev/null 2>&1 && ok=0 || ok=1
  result $ok "I06 a real install refuses to run without root"
fi
set -e

if [ $REAL = 0 ]; then
  echo
  echo "install lab (staged only): $PASS passed, $FAIL failed (of $N)"
  [ "$FAIL" = 0 ]
  exit
fi

# --- Part B: real install on a disposable machine ------------------------------
if [ "$(id -u)" != 0 ] || [ "$(cat /proc/1/comm)" != systemd ]; then
  echo "run.sh --real: needs root on a machine booted with systemd (disposable only)" >&2
  exit 2
fi
MEMBER="svim$$" OTHER="svio$$"
real_cleanup() {
  userdel "$MEMBER" 2>/dev/null || true
  userdel "$OTHER" 2>/dev/null || true
  "$PKG/uninstall.sh" --purge > /dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap real_cleanup EXIT
set +e
"$PKG/install.sh" --bindir "$BINDIR" --enable > "$WORK/b1.log" 2>&1; rc=$?
[ $rc = 0 ] && ok=1 || ok=0
result $ok "I10 real install with --enable succeeds (exit $rc)"
[ $ok = 1 ] || detail "$WORK/b1.log"
bad=$(for f in /usr/libexec/septuvo /usr/libexec/septuvo/septuvod /usr/bin/septuvo /usr/sbin/septuvo-server \
         /usr/lib/systemd/system/septuvod.service /usr/lib/systemd/system/septuvo-lab-netns.service \
         /usr/lib/systemd/system/septuvo-lockdown.service \
         /usr/lib/sysusers.d/septuvo.conf /usr/lib/tmpfiles.d/septuvo.conf /etc/septuvo /etc/septuvo/septuvod.toml; do
       [ "$(stat -c %U:%G "$f" 2>/dev/null)" = root:root ] || echo "$f"; done)
[ -z "$bad" ] && ok=1 || ok=0
result $ok "I10b every installed file is owned by root:root${bad:+ (not: $bad)}"
systemd-analyze verify /usr/lib/systemd/system/septuvod.service /usr/lib/systemd/system/septuvo-lab-netns.service \
  /usr/lib/systemd/system/septuvo-lockdown.service > "$WORK/verify.log" 2>&1 && ok=1 || ok=0
result $ok "I11 systemd-analyze verify accepts all three units"
[ $ok = 1 ] || detail "$WORK/verify.log"
for _ in $(seq 50); do [ -S /run/septuvo/status.sock ] && break; sleep 0.1; done
systemctl is-active --quiet septuvod.service && ip netns list | grep -qw septuvo-lab && ok=1 || ok=0
result $ok "I12 the service runs, inside the lab namespace septuvo-lab"
[ $ok = 1 ] || journalctl -u septuvod.service --no-pager -n 20 | detail
systemd-analyze security septuvod.service --no-pager 2>/dev/null | tail -1 | sed 's/^/      exposure: /'

uid=$(id -u septuvo-portal 2>/dev/null)
[ -n "$uid" ] && [ "$uid" != 0 ] && ok=1 || ok=0
result $ok "I12b the captive-portal helper account exists (uid ${uid:-none}, never root)"
useradd -M -N -s /usr/sbin/nologin -G septuvo "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
out=$(runuser -u "$OTHER" -- /usr/bin/septuvo status); rc=$?
[ $rc = 3 ] && grep -q "Status: Not connected" <<<"$out" && grep -q "Kill switch: off" <<<"$out" && ok=1 || ok=0
result $ok "I13 any user can read status after install; it never claims connected"
runuser -u "$OTHER" -- /usr/bin/septuvo connect > /dev/null 2>&1 && ok=0 || ok=1
result $ok "I14 a user outside the septuvo group cannot connect"
runuser -u "$MEMBER" -- /usr/bin/septuvo connect > "$WORK/connect.log" 2>&1 \
  && ip netns exec septuvo-lab nft list table inet septuvo > /dev/null 2>&1 && ok=1 || ok=0
result $ok "I15 a septuvo group member can connect: the kill switch is on in the lab namespace"
ip netns exec septuvo-lab nft list chain inet septuvo output 2>/dev/null | grep -q "meta skuid $uid tcp dport @captive_web accept" && ok=1 || ok=0
result $ok "I15b the installed service scopes captive-portal rules to the helper account only"
[ $ok = 1 ] || detail "$WORK/connect.log"
nft list table inet septuvo > /dev/null 2>&1 && ok=0 || ok=1
result $ok "I16 the host's own firewall is untouched (rules only in the lab namespace)"
systemctl stop septuvod.service
ip netns exec septuvo-lab nft list table inet septuvo > /dev/null 2>&1 && ok=1 || ok=0
result $ok "I17 stopping the service keeps the kill switch on (Q2)"
systemctl start septuvod.service
for _ in $(seq 50); do [ -S /run/septuvo/control/control.sock ] && break; sleep 0.1; done
runuser -u "$MEMBER" -- /usr/bin/septuvo disconnect > /dev/null 2>&1 \
  && ! ip netns exec septuvo-lab nft list table inet septuvo > /dev/null 2>&1 && ok=1 || ok=0
result $ok "I18 after a restart, a deliberate disconnect removes the rules"

# Lockdown Mode (N4-04, T2-LD-01 boot part).
before=$(systemctl show -p Before --value septuvo-lockdown.service)
nsbefore=$(systemctl show -p Before --value septuvo-lab-netns.service)
grep -qw network-pre.target <<<"$before" && grep -qw septuvod.service <<<"$before" \
  && grep -qw network-pre.target <<<"$nsbefore" \
  && [ "$(systemctl show -p DefaultDependencies --value septuvo-lockdown.service)" = no ] \
  && systemctl is-enabled --quiet septuvo-lockdown.service && ok=1 || ok=0
result $ok "I23 the Lockdown Mode boot unit is enabled and ordered before network-pre.target and the service"
[ $ok = 1 ] || echo "Before=$before; lab namespace Before=$nsbefore; enabled: $(systemctl is-enabled septuvo-lockdown.service)" | detail
svc() { runuser -u "$MEMBER" -- /usr/bin/septuvo "$@" 2>&1; }
# Waits until the started service answers status (up to 20 s).
ready() {
  for _ in $(seq 200); do
    /usr/bin/septuvo status 2>/dev/null | grep -q "^Service state:" && return 0
    sleep 0.1
  done
  return 1
}
svc lockdown on > "$WORK/ld.log"; rc=$?
st=$(svc status)
[ $rc = 0 ] && grep -q "Service state: lockdown" <<<"$st" && grep -q "Lockdown Mode: on" <<<"$st" \
  && ip netns exec septuvo-lab nft list table inet septuvo > /dev/null 2>&1 && ok=1 || ok=0
result $ok "I24 Lockdown Mode on: blocking rules installed with no connection"
[ $ok = 1 ] || { detail "$WORK/ld.log"; echo "$st" | detail; }
# A simulated reboot: the namespace (and every rule in it) goes away, then
# only the boot unit runs, as it does before the network at boot.
systemctl stop septuvod.service septuvo-lab-netns.service
ip netns list | grep -qw septuvo-lab && ok=0 || ok=1
systemctl start septuvo-lockdown.service > "$WORK/boot.log" 2>&1
ip netns exec septuvo-lab nft list table inet septuvo > /dev/null 2>&1 \
  && ! systemctl is-active --quiet septuvod.service || ok=0
result $ok "I25 at boot the Lockdown Mode rules are installed by the boot unit alone, before the service starts"
[ $ok = 1 ] || journalctl -u septuvo-lockdown.service --no-pager -n 20 | detail
systemctl start septuvod.service
ready
st=$(svc status)
grep -q "Service state: lockdown" <<<"$st" && ok=1 || ok=0
result $ok "I26 the service then reports Lockdown Mode blocking"
[ $ok = 1 ] || { echo "$st" | detail; journalctl -u septuvod.service --no-pager -n 8 | detail; }
systemctl stop septuvod.service
ip netns exec septuvo-lab /usr/libexec/septuvo/septuvod --settings /etc/septuvo/septuvod.toml --recover > "$WORK/recover.log" 2>&1; rc=$?
ip netns exec septuvo-lab nft list table inet septuvo > /dev/null 2>&1 && ok=0 || ok=1
systemctl start septuvod.service
ready
st=$(svc status)
[ $rc = 0 ] && grep -q "Lockdown Mode: off" <<<"$st" && grep -q "Service state: disconnected" <<<"$st" || ok=0
result $ok "I27 the administrator recovery command turns Lockdown Mode off and removes every rule"
[ $ok = 1 ] || { detail "$WORK/recover.log"; echo "$st" | detail; }
svc lockdown on > /dev/null

runuser -u "$MEMBER" -- /usr/bin/septuvo connect > /dev/null 2>&1
"$PKG/install.sh" --bindir "$BINDIR" > "$WORK/b2.log" 2>&1 && ok=1 || ok=0
result $ok "I19 re-running the installer on an installed system succeeds"
"$PKG/uninstall.sh" > "$WORK/un.log" 2>&1; rc=$?
left=""
for f in /usr/libexec/septuvo /usr/bin/septuvo /usr/sbin/septuvo-server \
         /usr/lib/systemd/system/septuvod.service /usr/lib/systemd/system/septuvo-lab-netns.service \
         /usr/lib/systemd/system/septuvo-lockdown.service \
         /usr/lib/sysusers.d/septuvo.conf /usr/lib/tmpfiles.d/septuvo.conf /usr/share/doc/septuvo \
         /run/septuvo /etc/netns/septuvo-lab; do
  [ -e "$f" ] && left="$left $f"
done
getent group septuvo > /dev/null && left="$left group:septuvo"
getent passwd septuvo-portal > /dev/null && left="$left user:septuvo-portal"
getent group septuvo-portal > /dev/null && left="$left group:septuvo-portal"
ip netns list | grep -qw septuvo-lab && left="$left netns:septuvo-lab"
systemctl list-unit-files 2>/dev/null | grep -q '^septuvo' && left="$left units"
[ $rc = 0 ] && [ -z "$left" ] && ok=1 || ok=0
result $ok "I20 uninstall (connected, Lockdown Mode on) leaves no files, units, group, namespace or rules${left:+ (left:$left)}"
[ $ok = 1 ] || detail "$WORK/un.log"
[ -f /etc/septuvo/septuvo.toml ] && grep -q "kept /etc/septuvo" "$WORK/un.log" && ok=1 || ok=0
result $ok "I21 uninstall keeps /etc/septuvo unless --purge, and says so"
"$PKG/uninstall.sh" --purge > /dev/null 2>&1
[ ! -e /etc/septuvo ] && ok=1 || ok=0
result $ok "I22 uninstall --purge removes /etc/septuvo"
set -e

echo
echo "install lab: $PASS passed, $FAIL failed (of $N)"
[ "$FAIL" = 0 ]
