#!/usr/bin/env bash
# Septuvo namespace lab (N4-04): Lockdown Mode (lab case T2-LD-01, D-038).
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/lockdown.sh [--require-ipv6] target/debug/septuvod target/debug/septuvo
#
# Two throw-away namespaces (C: the device, U: its network), two throw-away
# users and a private connection store under a temporary directory. With
# Lockdown Mode on, nothing may leave C except to a server endpoint: not
# with no connection, not after a deliberate disconnect, not while the
# service is killed, not after it restarts, and not at "boot" (a fresh
# namespace where only the start-up step has run). The rules must never be
# missing while connecting and disconnecting under load. Turning it off and
# the administrator recovery command must remove every rule. Nothing leaves
# the namespaces; everything is removed afterwards.
set -euo pipefail

REQUIRE_V6=0
if [ "${1:-}" = "--require-ipv6" ]; then REQUIRE_V6=1; shift; fi
usage="usage: lockdown.sh [--require-ipv6] PATH/TO/septuvod PATH/TO/septuvo"
DAEMON=$(realpath "${1:?$usage}")
CLI=$(realpath "${2:?$usage}")
HERE=$(cd "$(dirname "$0")" && pwd)

if [ "$(id -u)" != 0 ]; then
  echo "lockdown.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 runuser useradd userdel groupadd groupdel realpath; do
  command -v "$tool" >/dev/null || { echo "lockdown.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-lc-$ID" U="sv-lu-$ID"
GROUP="sv-lg-$ID" MEMBER="svlm$ID" OTHER="svlo$ID" HELPER="svlh$ID"
WORK=$(mktemp -d)
chmod 755 "$WORK"
RUN="$WORK/run"
STORE="$WORK/store/connections"
PASS=0 FAIL=0 SKIP=0 N=0
HAVE_V6=0
[ -d /proc/sys/net/ipv6 ] && HAVE_V6=1
DPID="" LPID="" FPID="" WPID=""

cleanup() {
  for p in "$FPID" "$WPID" "$LPID"; do [ -n "$p" ] && kill "$p" 2>/dev/null || true; done
  [ -n "$DPID" ] && { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null || true
  for ns in "$C" "$U"; do
    if ip netns list | grep -qw "$ns"; then
      ip netns pids "$ns" | xargs -r kill -9 2>/dev/null || true
      ip netns del "$ns" 2>/dev/null || true
    fi
  done
  userdel "$MEMBER" 2>/dev/null || true
  userdel "$OTHER" 2>/dev/null || true
  userdel "$HELPER" 2>/dev/null || true
  groupdel "$GROUP" 2>/dev/null || true
  groupdel "$HELPER" 2>/dev/null || true
  rm -rf "/etc/netns/$C" "/etc/netns/.septuvo-$C.resolv.conf.backup" "/etc/netns/.septuvo-$C.resolv.conf.tmp" "$WORK"
}
trap cleanup EXIT
trap 'echo "::error title=lockdown lab setup::lockdown.sh line $LINENO failed: $BASH_COMMAND"' ERR

result() {   # $1: 1 pass, 0 fail, skip; $2: description
  N=$((N + 1))
  case "$1" in
    1) PASS=$((PASS + 1)); echo "PASS  $2" ;;
    skip)
      if [ $REQUIRE_V6 = 1 ]; then
        FAIL=$((FAIL + 1)); echo "FAIL  $2 (IPv6 required but unavailable)"
      else
        SKIP=$((SKIP + 1)); echo "SKIP  $2 (no IPv6 in this kernel)"
      fi ;;
    *)
      FAIL=$((FAIL + 1)); echo "FAIL  $2"
      if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=lockdown lab::$2"; fi
      if [ -f "$WORK/daemon.log" ]; then sed 's/^/      daemon: /' "$WORK/daemon.log" | tail -5; fi ;;
  esac
}

inC() { ip netns exec "$C" "$@"; }
sv() { runuser -u "$MEMBER" -- "$WORK/bin/septuvo" "$@" --runtime-dir "$RUN"; }
table() { inC nft list table inet septuvo 2>/dev/null; }

groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
# A captive-portal helper account, as installed: the Lockdown Mode ruleset
# then carries the helper's (empty) window rules too.
useradd -r -M -U -s /usr/sbin/nologin "$HELPER"
mkdir "$WORK/bin" "$WORK/store"
chmod 700 "$WORK/store"
install -m 755 "$DAEMON" "$WORK/bin/septuvod"
install -m 755 "$CLI" "$WORK/bin/septuvo"

for ns in "$C" "$U"; do
  ip netns add "$ns"
  ip -n "$ns" link set lo up
done
ip link add eth0 netns "$C" type veth peer name c0 netns "$U"
ip -n "$C" addr add 10.200.0.10/24 dev eth0
ip -n "$U" addr add 10.200.0.1/24 dev c0
LISTEN=(10.200.0.1)
if [ $HAVE_V6 = 1 ]; then
  ip -n "$C" addr add 2001:db8:200::10/64 dev eth0 nodad
  ip -n "$U" addr add 2001:db8:200::1/64 dev c0 nodad
  LISTEN+=(2001:db8:200::1)
fi
ip -n "$C" link set eth0 up
ip -n "$U" link set c0 up
ip -n "$C" route add default via 10.200.0.1

ip netns exec "$U" python3 "$HERE/probe.py" listen "$WORK/u.log" "$WORK/u.ready" "${LISTEN[@]}" &
LPID=$!
for _ in $(seq 50); do [ -f "$WORK/u.ready" ] && break; sleep 0.1; done

# The settings' own configuration: server 10.200.0.1:5000. A saved
# connection (added later) uses 10.200.0.1:443.
cat > "$WORK/septuvo.toml" <<'EOF'
version = 1
[server]
endpoints = ["10.200.0.1:5000"]
[routing]
mode = "full-tunnel"
[dns]
resolvers = ["10.77.0.1"]
EOF
sed 's/10.200.0.1:5000/10.200.0.1:443/' "$WORK/septuvo.toml" > "$WORK/b.toml"
cat > "$WORK/septuvod.toml" <<EOF
version = 1
[daemon]
mode = "lab"
lab_namespace = "$C"
runtime_dir = "$RUN"
control_group = "$GROUP"
device_config = "$WORK/septuvo.toml"
connections_dir = "$STORE"
captive_helper_user = "$HELPER"
EOF
chmod 644 "$WORK"/*.toml

start_daemon() {
  : > "$WORK/daemon.log"
  inC "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
  DPID=$!
  for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && return 0; sleep 0.1; done
  return 1
}
kill_daemon() { { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null; DPID=""; }
send() { inC python3 "$HERE/probe.py" send "$1" "$2" "$3" "$4"; }
arrived() { sleep 0.3; grep -qx "$1" "$WORK/u.log" 2>/dev/null; }
# Everything except the server endpoint is blocked: TCP and UDP to other
# ports, DNS (UDP and TCP 53, TCP 853), and IPv6 when available.
blocked_all() {   # $1: token prefix
  local p="$1" ok=1
  [ "$(send udp 10.200.0.1 53 "$p-u53")" = sent ] && ok=0
  [ "$(send tcp 10.200.0.1 53 "$p-t53")" = sent ] && ok=0
  [ "$(send tcp 10.200.0.1 853 "$p-t853")" = sent ] && ok=0
  [ "$(send tcp 10.200.0.1 80 "$p-t80")" = sent ] && ok=0
  [ "$(send udp 10.200.0.1 67 "$p-u67")" = sent ] && ok=0
  if [ $HAVE_V6 = 1 ]; then
    [ "$(send udp 2001:db8:200::1 53 "$p-v6u53")" = sent ] && ok=0
    [ "$(send tcp 2001:db8:200::1 80 "$p-v6t80")" = sent ] && ok=0
  fi
  sleep 0.3
  grep -q "^$p-" "$WORK/u.log" 2>/dev/null && ok=0
  echo $ok
}

trap - ERR
set +e

start_daemon && ok=1 || ok=0
result $ok "L00 septuvod starts with an empty private store"
st=$(sv status)
grep -q "Lockdown Mode: off" <<<"$st" && grep -q "Service state: disconnected" <<<"$st" \
  && [ -z "$(table)" ] && [ "$(send tcp 10.200.0.1 80 tok-open)" = sent ] && arrived tok-open && ok=1 || ok=0
result $ok "L01 Lockdown Mode is off by default: no rules, traffic flows"
runuser -u "$OTHER" -- "$WORK/bin/septuvo" lockdown on --runtime-dir "$RUN" >/dev/null 2>&1 && ok=0 || ok=1
[ -z "$(table)" ] || ok=0
result $ok "L02 a user outside the control group cannot turn Lockdown Mode on"
sv lockdown maybe >/dev/null 2>&1; [ $? = 2 ] && ok=1 || ok=0
result $ok "L03 an invalid value is a usage error and changes nothing"

out=$(sv lockdown on 2>&1); rc=$?
st=$(sv status)
[ $rc = 0 ] && grep -q "Service state: lockdown" <<<"$st" && grep -q "Lockdown Mode: on" <<<"$st" && ok=1 || ok=0
result $ok "L04 turning Lockdown Mode on with no connection: state lockdown ($out)"
[ "$(blocked_all ld1)" = 1 ] && ok=1 || ok=0
result $ok "T2-LD-01a with no connection everything is blocked (UDP/TCP 53, TCP 853, other ports, IPv6)"
[ "$(send udp 10.200.0.1 5000 tok-ep1)" = sent ] && arrived tok-ep1 && ok=1 || ok=0
result $ok "L05 the server endpoint stays reachable, so a connection can be made"

# Connect and disconnect under load: the rules are never missing and
# nothing outside the endpoint gets out.
rm -f "$WORK/stop" "$WORK/watch.stop"
inC python3 "$HERE/probe.py" flood 10.200.0.1 "$WORK/stop" flood 53 67 443 > "$WORK/flood.txt" &
FPID=$!
( while [ ! -f "$WORK/watch.stop" ]; do
    if inC nft list table inet septuvo >/dev/null 2>&1; then echo present; else echo MISSING; fi
  done > "$WORK/presence.txt" ) &
WPID=$!
sleep 0.5
cycles=0
for _ in $(seq 10); do
  sv connect > "$WORK/c.txt" 2>&1 && grep -q "Service state: blocking" "$WORK/c.txt" \
    && sv disconnect > "$WORK/d.txt" 2>&1 && grep -q "Service state: lockdown" "$WORK/d.txt" && cycles=$((cycles + 1))
done
sleep 0.5
touch "$WORK/stop" "$WORK/watch.stop"
wait "$FPID" 2>/dev/null; FPID=""
wait "$WPID" 2>/dev/null; WPID=""
[ "$cycles" = 10 ] && ok=1 || ok=0
result $ok "L06 10 connect/disconnect cycles: each disconnect ends in Lockdown Mode ($cycles/10)"
leaked=$(grep -c '^flood-' "$WORK/u.log")
[ "$leaked" = 0 ] || grep '^flood-' "$WORK/u.log" | head -5 | sed 's/^/      leaked: /'
refused=$(awk '{s += $3} END {print s + 0}' "$WORK/flood.txt")
missing=$(grep -c MISSING "$WORK/presence.txt")
checks=$(wc -l < "$WORK/presence.txt")
[ "$leaked" = 0 ] && [ "$refused" -gt 1000 ] && [ "$missing" = 0 ] && [ "$checks" -gt 10 ] && ok=1 || ok=0
result $ok "T2-LD-01b after each deliberate disconnect blocking continues: $leaked leaked, $refused refused; rules missing in $missing of $checks checks"

kill_daemon
[ -n "$(table)" ] && [ "$(blocked_all ld2)" = 1 ] && ok=1 || ok=0
result $ok "T2-LD-01c with the service killed, everything stays blocked"
start_daemon
grep -q "Service state: lockdown" <<<"$(sv status)" && [ "$(blocked_all ld3)" = 1 ] && ok=1 || ok=0
result $ok "T2-LD-01d after a service restart: still Lockdown Mode, still blocked"

# "Boot": the namespace starts with no rules; only the start-up step runs.
kill_daemon
inC nft delete table inet septuvo
[ "$(send tcp 10.200.0.1 80 tok-gap)" = sent ] && gap=1 || gap=0
out=$(inC "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" --apply-lockdown 2>&1); rc=$?
[ $gap = 1 ] && [ $rc = 0 ] && grep -q "blocking rules installed" <<<"$out" && [ -n "$(table)" ] \
  && [ "$(blocked_all ld4)" = 1 ] && ok=1 || ok=0
result $ok "T2-LD-01e at boot the start-up step alone installs the blocking rules ($out)"
start_daemon
grep -q "Service state: lockdown" <<<"$(sv status)" && ok=1 || ok=0
result $ok "L07 the service started after the boot step reports Lockdown Mode"

inC nft delete table inet septuvo
st=$(sv status)
grep -q "Service state: error-lockdown-missing" <<<"$st" && ok=1 || ok=0
result $ok "L08 rules removed by hand: shown as an error, never as plain disconnected"
sv lockdown on >/dev/null 2>&1
[ -n "$(table)" ] && grep -q "Service state: lockdown" <<<"$(sv status)" && ok=1 || ok=0
result $ok "L09 turning Lockdown Mode on again re-installs the rules"

sv connection add "$WORK/b.toml" --name "Office" >/dev/null 2>&1
[ "$(send udp 10.200.0.1 443 tok-ep2)" = sent ] && arrived tok-ep2 && ok=1 || ok=0
result $ok "L10 a newly saved connection's server becomes reachable in Lockdown Mode"
sv connection delete c0001 >/dev/null 2>&1
[ "$(send udp 10.200.0.1 443 tok-ep3)" = sent ] && ok=0 || ok=1
result $ok "L11 a deleted connection's server is blocked again"

out=$(sv lockdown off 2>&1); rc=$?
[ $rc = 0 ] && [ -z "$(table)" ] && grep -q "Lockdown Mode: off" <<<"$(sv status)" \
  && [ "$(send tcp 10.200.0.1 80 tok-off)" = sent ] && arrived tok-off && ok=1 || ok=0
result $ok "L12 turning Lockdown Mode off removes every rule; traffic flows again"
sv connect >/dev/null 2>&1
sv disconnect >/dev/null 2>&1
[ -z "$(table)" ] && ok=1 || ok=0
result $ok "L13 with Lockdown Mode off a deliberate disconnect removes the rules (Q2 unchanged)"

# Administrator recovery: service stopped, Lockdown Mode on.
sv lockdown on >/dev/null 2>&1
kill_daemon
out=$(inC "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" --recover 2>&1); rc=$?
[ $rc = 0 ] && [ -z "$(table)" ] && ok=1 || ok=0
start_daemon
st=$(sv status)
grep -q "Lockdown Mode: off" <<<"$st" && grep -q "Service state: disconnected" <<<"$st" && [ -z "$(table)" ] || ok=0
result $ok "L14 the recovery command turns Lockdown Mode off and removes every rule; it stays off after a restart ($out)"
"$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" --recover >/dev/null 2>&1; [ $? = 3 ] && ok=1 || ok=0
result $ok "L15 the recovery command refuses to run outside the lab namespace"
if [ $HAVE_V6 = 1 ]; then
  sv lockdown on >/dev/null 2>&1
  [ "$(send udp 2001:db8:200::1 5000 tok-v6)" = sent ] && ok=0 || ok=1
  sv lockdown off >/dev/null 2>&1
  result $ok "L16 IPv6 is blocked too (no IPv6 endpoint configured)"
else
  result skip "L16 IPv6 is blocked too (no IPv6 endpoint configured)"
fi
set -e

echo
echo "lockdown lab: $PASS passed, $FAIL failed, $SKIP skipped (of $N)"
[ "$FAIL" = 0 ]
