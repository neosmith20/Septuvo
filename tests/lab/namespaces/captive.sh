#!/usr/bin/env bash
# Septuvo namespace lab (N3-02): captive-portal mode (Q1).
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/captive.sh [--require-ipv6] target/debug/septuvod target/debug/septuvo
#
# Namespaces (all throw-away):
#   C  the device: runs septuvod (lab mode), full tunnel, tunnel DNS
#   U  a hotel network: router and network resolver 192.168.1.1 (the
#      device's original resolver file), a sign-in page 192.168.1.80, another
#      LAN DNS server .54, an internet host 203.0.113.80 and a host the policy
#      blocks, 203.0.113.99
#   T  the far end of a stand-in tunnel interface septuvo0 (no encryption,
#      no SVP), with the tunnel resolver 10.77.0.1
# One 1-minute window covers the timeout AND a service killed during the
# window: the kernel must close it on time either way.
set -euo pipefail

REQUIRE_V6=0
if [ "${1:-}" = "--require-ipv6" ]; then REQUIRE_V6=1; shift; fi
DAEMON=$(realpath "${1:?usage: captive.sh [--require-ipv6] PATH/TO/septuvod PATH/TO/septuvo}")
CLI=$(realpath "${2:?usage: captive.sh [--require-ipv6] PATH/TO/septuvod PATH/TO/septuvo}")
HERE=$(cd "$(dirname "$0")" && pwd)
PROBE="$HERE/probe.py"

if [ "$(id -u)" != 0 ]; then
  echo "captive.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 runuser useradd userdel groupadd groupdel realpath; do
  command -v "$tool" >/dev/null || { echo "captive.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-pc-$ID" U="sv-pu-$ID" T="sv-pt-$ID"
GROUP="sv-cp-$ID" MEMBER="svcm$ID" OTHER="svco$ID"
WORK=$(mktemp -d)
chmod 755 "$WORK"
RUN="$WORK/run"
RESOLV_DIR="/etc/netns/$C"
PASS=0 FAIL=0 SKIP=0 N=0
HAVE_V6=0
[ -d /proc/sys/net/ipv6 ] && HAVE_V6=1
v6() { if [ $HAVE_V6 = 1 ]; then "$@"; fi; }
DPID=""

cleanup() {
  [ -n "$DPID" ] && { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null || true
  for ns in "$C" "$U" "$T"; do
    if ip netns list | grep -qw "$ns"; then
      ip netns pids "$ns" | xargs -r kill -9 2>/dev/null || true
      ip netns del "$ns" 2>/dev/null || true
    fi
  done
  userdel "$MEMBER" 2>/dev/null || true
  userdel "$OTHER" 2>/dev/null || true
  groupdel "$GROUP" 2>/dev/null || true
  rm -rf "$RESOLV_DIR" "$WORK" "/etc/netns/.septuvo-$C.resolv.conf.backup" "/etc/netns/.septuvo-$C.resolv.conf.tmp"
}
trap cleanup EXIT
trap 'echo "::error title=captive lab setup::captive.sh line $LINENO failed: $BASH_COMMAND"' ERR

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
      if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=captive lab::$2"; fi
      if [ -f "$WORK/daemon.log" ]; then sed 's/^/      daemon: /' "$WORK/daemon.log" | tail -5; fi ;;
  esac
}
inC() { ip netns exec "$C" "$@"; }
as() { local who=$1; shift; runuser -u "$who" -- "$@"; }

# --- users, binaries, namespaces ---------------------------------------------
groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
mkdir "$WORK/bin"
install -m 755 "$DAEMON" "$WORK/bin/septuvod"
install -m 755 "$CLI" "$WORK/bin/septuvo"

for ns in "$C" "$U" "$T"; do
  ip netns add "$ns"
  ip -n "$ns" link set lo up
  if [ $HAVE_V6 = 1 ]; then
    ip netns exec "$ns" sysctl -q -w net.ipv6.conf.all.accept_dad=0 net.ipv6.conf.default.accept_dad=0
  fi
done
ip link add eth0 netns "$C" type veth peer name c0 netns "$U"
ip -n "$C" addr add 192.168.1.10/24 dev eth0
v6 ip -n "$C" addr add 2001:db8:1::10/64 dev eth0 nodad
for host in 1 54 80; do
  ip -n "$U" addr add "192.168.1.$host/24" dev c0
  v6 ip -n "$U" addr add "2001:db8:1::$host/64" dev c0 nodad
done
ip -n "$U" addr add 203.0.113.80/32 dev c0
ip -n "$U" addr add 203.0.113.99/32 dev c0
ip -n "$C" link set eth0 up
ip -n "$U" link set c0 up
ip -n "$C" route add default via 192.168.1.1
v6 ip -n "$C" -6 route add default via 2001:db8:1::1
ip link add septuvo0 netns "$C" type veth peer name t0 netns "$T"
ip -n "$C" addr add 10.77.0.2/24 dev septuvo0
ip -n "$T" addr add 10.77.0.1/24 dev t0
ip -n "$C" link set septuvo0 up
ip -n "$T" link set t0 up
if [ $HAVE_V6 = 1 ]; then
  for _ in $(seq 20); do
    ip -n "$C" -6 addr show tentative | grep -q inet6 || break
    sleep 0.1
  done
fi

U_ADDRS="192.168.1.1 192.168.1.54 192.168.1.80 203.0.113.80 203.0.113.99"
[ $HAVE_V6 = 1 ] && U_ADDRS="$U_ADDRS 2001:db8:1::1 2001:db8:1::80"
# shellcheck disable=SC2086
ip netns exec "$U" python3 "$PROBE" listen "$WORK/u.log" "$WORK/u.ready" $U_ADDRS &
ip netns exec "$T" python3 "$PROBE" listen "$WORK/t.log" "$WORK/t.ready" 10.77.0.1 &
for _ in $(seq 50); do [ -f "$WORK/u.ready" ] && [ -f "$WORK/t.ready" ] && break; sleep 0.1; done

cat > "$WORK/septuvo.toml" <<EOF
version = 1
[server]
endpoints = ["192.168.1.1:5000"]
[routing]
mode = "full-tunnel"
block = ["203.0.113.99/32"]
[dns]
resolvers = ["10.77.0.1"]
EOF
cat > "$WORK/septuvod.toml" <<EOF
version = 1
[daemon]
mode = "lab"
lab_namespace = "$C"
runtime_dir = "$RUN"
control_group = "$GROUP"
device_config = "$WORK/septuvo.toml"
EOF
chmod 644 "$WORK"/*.toml
mkdir -p "$RESOLV_DIR"
{ echo "nameserver 192.168.1.1"; [ $HAVE_V6 = 1 ] && echo "nameserver 2001:db8:1::1"; } > "$RESOLV_DIR/resolv.conf"

start_daemon() {
  : > "$WORK/daemon.log"
  inC "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
  DPID=$!
  for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && return 0; sleep 0.1; done
  return 1
}
kill_daemon() { { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null; DPID=""; }
cli() { local who=$1; shift; as "$who" "$WORK/bin/septuvo" "$@" --runtime-dir "$RUN"; }

# check DESCRIPTION PROTO DST PORT WHERE (u, t or none)
check() {
  local desc=$1 proto=$2 dst=$3 port=$4 where=$5 token ok
  N_TOKEN=$((${N_TOKEN:-0} + 1)); token="cp$ID-$N_TOKEN"
  inC python3 "$PROBE" send "$proto" "$dst" "$port" "$token" > /dev/null
  sleep 0.2
  case "$where" in
    none) ! grep -qx "$token" "$WORK/u.log" "$WORK/t.log" 2>/dev/null && ok=1 || ok=0 ;;
    *) grep -qx "$token" "$WORK/$where.log" && ok=1 || ok=0 ;;
  esac
  result $ok "$desc"
}
check6() { if [ $HAVE_V6 = 1 ]; then check "$@"; else result skip "$1"; fi; }

# Setup is done; from here on, failing commands are expected test outcomes.
trap - ERR
set +e
start_daemon && ok=1 || ok=0
result $ok "CP00 septuvod starts in the lab namespace"
out=$(cli "$MEMBER" captive-portal); rc=$?
[ $rc = 1 ] && grep -q "only available while connected" <<<"$out" && ok=1 || ok=0
result $ok "CP01 captive-portal mode is refused while not connected (exit $rc)"
cli "$MEMBER" connect > /dev/null && ok=1 || ok=0
result $ok "CP02 connect succeeds (kill switch on)"

check  "CP03 before: the sign-in page (TCP 80) outside the tunnel is blocked" tcp 192.168.1.80 80 none
check  "CP04 before: HTTPS (TCP 443) outside the tunnel is blocked" tcp 203.0.113.80 443 none
check  "CP05 before: DNS to the network's resolver is blocked" udp 192.168.1.1 53 none

out=$(cli "$OTHER" captive-portal --minutes 1); rc=$?
[ $rc != 0 ] && ok=1 || ok=0
check  "CP06 a user outside the control group cannot open it (exit $rc): page still blocked" tcp 192.168.1.80 80 none
[ $ok = 1 ] || result 0 "CP06b the non-member command must fail"

out=$(cli "$MEMBER" captive-portal --minutes 1); rc=$?
OPENED=$(date +%s)
[ $rc = 0 ] && grep -q "Captive portal: OPEN (" <<<"$out" && grep -q "WARNING: captive-portal mode" <<<"$out" \
  && grep -q "network DNS: 192.168.1.1" <<<"$out" && ok=1 || ok=0
result $ok "CP07 a member opens a 1-minute window; the warning and countdown are shown (exit $rc)"
[ $ok = 1 ] || sed 's/^/      /' <<<"$out"

check  "CP08 during: the sign-in page (TCP 80) goes out directly" tcp 192.168.1.80 80 u
check  "CP09 during: HTTPS (TCP 443) goes out directly" tcp 203.0.113.80 443 u
check  "CP10 during: UDP DNS to the network's resolver goes out directly" udp 192.168.1.1 53 u
check  "CP11 during: TCP DNS to the network's resolver goes out directly" tcp 192.168.1.1 53 u
check  "CP12 during: DNS to another LAN DNS server stays blocked" udp 192.168.1.54 53 none
check  "CP13 during: DNS over TLS (853) to the network's resolver stays blocked" tcp 192.168.1.1 853 none
check  "CP14 during: QUIC (UDP 443) stays blocked" udp 203.0.113.80 443 none
check  "CP15 during: other TCP ports stay blocked" tcp 203.0.113.80 5000 none
check  "CP16 during: other UDP stays blocked" udp 203.0.113.80 5000 none
check  "CP17 during: a host the policy blocks stays blocked, even on TCP 80" tcp 203.0.113.99 80 none
check  "CP18 during: tunnel traffic still uses the tunnel" udp 10.77.0.1 53 t
check6 "CP19 during: IPv6 sign-in page (TCP 80) goes out directly" tcp 2001:db8:1::80 80 u
check6 "CP20 during: IPv6 DNS to the network's resolver goes out directly" udp 2001:db8:1::1 53 u
out=$(cli "$OTHER" status)
grep -q "Captive portal: OPEN" <<<"$out" && grep -q "WARNING:" <<<"$out" && grep -q "Status: Not connected" <<<"$out" && ok=1 || ok=0
result $ok "CP21 status shows the open window and the warning, and still never claims connected"

kill_daemon
check  "CP22 with the service killed mid-window, the window is still the kernel's: page reachable" tcp 192.168.1.80 80 u
start_daemon > /dev/null
out=$(cli "$OTHER" status)
grep -q "time left unknown; it still ends on time" <<<"$out" && ok=1 || ok=0
result $ok "CP23 a restarted service reports the window it did not open (time left unknown)"
kill_daemon

WAIT=$((OPENED + 63 - $(date +%s)))
echo "      waiting ${WAIT}s for the kernel to end the window (service NOT running)"
[ $WAIT -gt 0 ] && sleep $WAIT
check  "CP24 after the timeout, with no service running: the sign-in page is blocked again" tcp 192.168.1.80 80 none
check  "CP25 after the timeout: HTTPS outside the tunnel is blocked again" tcp 203.0.113.80 443 none
check  "CP26 after the timeout: DNS to the network's resolver is blocked again" udp 192.168.1.1 53 none
check6 "CP27 after the timeout: IPv6 sign-in page is blocked again" tcp 2001:db8:1::80 80 none
start_daemon > /dev/null
out=$(cli "$OTHER" status)
grep -q "Captive portal: closed" <<<"$out" && grep -q "Kill switch: on" <<<"$out" && ok=1 || ok=0
result $ok "CP28 status after the timeout: closed, kill switch still on"

out=$(cli "$MEMBER" captive-portal); rc=$?
[ $rc = 1 ] && grep -q "only available while connected" <<<"$out" && ok=1 || ok=0
result $ok "CP28b after a restart, rules left from the earlier run must be reconnected first (exit $rc)"
cli "$MEMBER" connect > /dev/null
cli "$MEMBER" captive-portal > /dev/null
cli "$MEMBER" captive-portal --minutes 1 > /dev/null
set_text=$(inC nft list set inet septuvo captive_web)
grep -q "80 timeout 1m" <<<"$set_text" && ! grep -q "timeout 5m" <<<"$set_text" && ! grep -q "timeout 6m" <<<"$set_text" && ok=1 || ok=0
result $ok "CP29 asking again restarts the window from now; it never adds up"
[ $ok = 1 ] || sed 's/^/      /' <<<"$set_text"
cli "$MEMBER" disconnect > /dev/null
! inC nft list table inet septuvo > /dev/null 2>&1 && ok=1 || ok=0
result $ok "CP30 disconnect ends the window at once (no Septuvo rules left)"
cli "$MEMBER" connect > /dev/null
check  "CP31 reconnecting starts with the window closed" tcp 192.168.1.80 80 none
cli "$MEMBER" disconnect > /dev/null

echo
echo "captive lab: $PASS passed, $FAIL failed, $SKIP skipped (of $N; IPv6 available: $HAVE_V6)"
[ "$FAIL" = 0 ]
