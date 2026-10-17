#!/usr/bin/env bash
# Septuvo namespace lab (N2-03): DNS lockdown and the resolver file.
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/dns.sh [--require-ipv6] target/debug/septuvod target/debug/septuvo
#
# Namespaces (all throw-away):
#   C  the device: runs septuvod (lab mode) with tunnel DNS
#   U  its local network: router/endpoint 192.168.1.1, an approved LAN DNS
#      server .53 (a configured exception), another LAN DNS server .54, and an
#      "internet" DNS server 203.0.113.53
#   T  the far end of a stand-in tunnel interface septuvo0 (no encryption, no
#      SVP): the tunnel resolver 10.77.0.1 / fd77::1, plus 198.51.100.53, a
#      DNS server reachable through the tunnel that is NOT a configured resolver
# Every probe sends a unique token and checks where it arrives.
set -euo pipefail

REQUIRE_V6=0
if [ "${1:-}" = "--require-ipv6" ]; then REQUIRE_V6=1; shift; fi
DAEMON=$(realpath "${1:?usage: dns.sh [--require-ipv6] PATH/TO/septuvod PATH/TO/septuvo}")
CLI=$(realpath "${2:?usage: dns.sh [--require-ipv6] PATH/TO/septuvod PATH/TO/septuvo}")
HERE=$(cd "$(dirname "$0")" && pwd)
PROBE="$HERE/probe.py"

if [ "$(id -u)" != 0 ]; then
  echo "dns.sh: needs root (network namespaces); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 realpath; do
  command -v "$tool" >/dev/null || { echo "dns.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-nc-$ID" U="sv-nu-$ID" T="sv-nt-$ID"
WORK=$(mktemp -d)
RUN="$WORK/run"
RESOLV_DIR="/etc/netns/$C"
RESOLV="$RESOLV_DIR/resolv.conf"
BACKUP="/etc/netns/.septuvo-$C.resolv.conf.backup"
ORIGINAL="nameserver 192.168.1.54"
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
  rm -rf "$RESOLV_DIR" "$WORK" "$BACKUP" "/etc/netns/.septuvo-$C.resolv.conf.tmp"
}
trap cleanup EXIT
trap 'echo "::error title=dns lab setup::dns.sh line $LINENO failed: $BASH_COMMAND"' ERR

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
      if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=dns lab::$2"; fi ;;
  esac
}
inC() { ip netns exec "$C" "$@"; }

# --- namespaces --------------------------------------------------------------
for ns in "$C" "$U" "$T"; do
  ip netns add "$ns"
  ip -n "$ns" link set lo up
  if [ $HAVE_V6 = 1 ]; then
    ip netns exec "$ns" sysctl -q -w net.ipv6.conf.all.accept_dad=0 net.ipv6.conf.default.accept_dad=0
  fi
done
# Underlay: C eth0 <-> U c0
ip link add eth0 netns "$C" type veth peer name c0 netns "$U"
ip -n "$C" addr add 192.168.1.10/24 dev eth0
v6 ip -n "$C" addr add 2001:db8:1::10/64 dev eth0 nodad
ip -n "$U" addr add 192.168.1.1/24 dev c0
v6 ip -n "$U" addr add 2001:db8:1::1/64 dev c0 nodad
for host in 53 54; do
  ip -n "$U" addr add "192.168.1.$host/24" dev c0
  v6 ip -n "$U" addr add "2001:db8:1::$host/64" dev c0 nodad
done
ip -n "$U" addr add 203.0.113.53/32 dev c0
v6 ip -n "$U" addr add 2001:db8:53::53/128 dev c0 nodad
ip -n "$C" link set eth0 up
ip -n "$U" link set c0 up
ip -n "$C" route add default via 192.168.1.1
v6 ip -n "$C" -6 route add default via 2001:db8:1::1
# Stand-in tunnel: C septuvo0 <-> T t0
ip link add septuvo0 netns "$C" type veth peer name t0 netns "$T"
ip -n "$C" addr add 10.77.0.2/24 dev septuvo0
v6 ip -n "$C" addr add fd77::2/64 dev septuvo0 nodad
ip -n "$T" addr add 10.77.0.1/24 dev t0
v6 ip -n "$T" addr add fd77::1/64 dev t0 nodad
ip -n "$T" addr add 198.51.100.53/32 dev t0
v6 ip -n "$T" addr add 2001:db8:99::53/128 dev t0 nodad
ip -n "$C" link set septuvo0 up
ip -n "$T" link set t0 up
ip -n "$C" route add 198.51.100.53/32 via 10.77.0.1 dev septuvo0
v6 ip -n "$C" -6 route add 2001:db8:99::53/128 via fd77::1 dev septuvo0
if [ $HAVE_V6 = 1 ]; then
  for _ in $(seq 20); do
    ip -n "$C" -6 addr show tentative | grep -q inet6 || break
    sleep 0.1
  done
fi

# Listeners (UDP 53, TCP 53 and 853, UDP/TCP 5000 on each address).
U_ADDRS="192.168.1.1 192.168.1.53 192.168.1.54 203.0.113.53"
T_ADDRS="10.77.0.1 198.51.100.53"
if [ $HAVE_V6 = 1 ]; then
  U_ADDRS="$U_ADDRS 2001:db8:1::53 2001:db8:1::54 2001:db8:53::53"
  T_ADDRS="$T_ADDRS fd77::1 2001:db8:99::53"
fi
# shellcheck disable=SC2086
ip netns exec "$U" python3 "$PROBE" listen "$WORK/u.log" "$WORK/u.ready" $U_ADDRS &
# shellcheck disable=SC2086
ip netns exec "$T" python3 "$PROBE" listen "$WORK/t.log" "$WORK/t.ready" $T_ADDRS &
for _ in $(seq 50); do [ -f "$WORK/u.ready" ] && [ -f "$WORK/t.ready" ] && break; sleep 0.1; done

# --- configuration -------------------------------------------------------------
DIRECT='"192.168.1.53/32"'
LAN_DNS='"192.168.1.53"'
RESOLVERS='"10.77.0.1"'
if [ $HAVE_V6 = 1 ]; then
  DIRECT="$DIRECT, \"2001:db8:1::53/128\""
  LAN_DNS="$LAN_DNS, \"2001:db8:1::53\""
  RESOLVERS="$RESOLVERS, \"fd77::1\""
fi
cat > "$WORK/septuvo.toml" <<EOF
version = 1
[server]
endpoints = ["192.168.1.1:5000"]
[routing]
mode = "full-tunnel"
direct = [$DIRECT]
[dns]
resolvers = [$RESOLVERS]
lan_exceptions = [$LAN_DNS]
EOF
cat > "$WORK/septuvod.toml" <<EOF
version = 1
[daemon]
mode = "lab"
lab_namespace = "$C"
runtime_dir = "$RUN"
control_group = "root"
device_config = "$WORK/septuvo.toml"
EOF
mkdir -p "$RESOLV_DIR"
echo "$ORIGINAL" > "$RESOLV"

start_daemon() {
  : > "$WORK/daemon.log"
  inC "$DAEMON" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
  DPID=$!
  for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && return 0; sleep 0.1; done
  cat "$WORK/daemon.log" >&2
  return 1
}
cli() { "$CLI" "$1" --runtime-dir "$RUN"; }

# probe PROTO DST PORT TOKEN WHERE: WHERE is u, t (expect arrival) or none.
probe() {
  local proto=$1 dst=$2 port=$3 token=$4 where=$5 r
  r=$(inC python3 "$PROBE" send "$proto" "$dst" "$port" "$token")
  sleep 0.2
  case "$where" in
    none) ! grep -qx "$token" "$WORK/u.log" "$WORK/t.log" 2>/dev/null ;;
    *) grep -qx "$token" "$WORK/$where.log" ;;
  esac
}
check() {   # check DESCRIPTION PROTO DST PORT WHERE
  local desc=$1; shift
  N_TOKEN=$((${N_TOKEN:-0} + 1))
  if probe "$1" "$2" "$3" "tok$N_TOKEN" "$4"; then result 1 "$desc"; else result 0 "$desc"; fi
}
check6() {
  if [ $HAVE_V6 = 1 ]; then check "$@"; else result skip "$1"; fi
}

# Setup is done; from here on, failing commands are expected test outcomes.
trap - ERR
set +e
start_daemon && result 1 "DN00 septuvod starts in the lab namespace" || result 0 "DN00 septuvod starts in the lab namespace"
check "DN01 before connect, DNS to an internet resolver works normally" udp 203.0.113.53 53 u

cli connect > "$WORK/connect.out" && ok=1 || ok=0
result $ok "DN02 connect succeeds"

check  "DN03 UDP DNS to the tunnel resolver goes through the tunnel" udp 10.77.0.1 53 t
check  "DN04 TCP DNS to the tunnel resolver goes through the tunnel" tcp 10.77.0.1 53 t
check  "DN05 DNS over TLS (TCP 853) to the tunnel resolver goes through the tunnel" tcp 10.77.0.1 853 t
check  "DN06 UDP DNS to an internet resolver outside the tunnel is blocked" udp 203.0.113.53 53 none
check  "DN07 TCP DNS to an internet resolver outside the tunnel is blocked" tcp 203.0.113.53 53 none
check  "DN08 DNS over TLS to an internet resolver is blocked" tcp 203.0.113.53 853 none
check  "DN09 DNS to a non-resolver THROUGH the tunnel is blocked (resolvers only)" udp 198.51.100.53 53 none
check  "DN10 DNS to the configured LAN exception is allowed directly" udp 192.168.1.53 53 u
check  "DN11 DNS to another LAN DNS server is blocked" udp 192.168.1.54 53 none
check  "DN12 the LAN exception host still gets non-DNS traffic (direct rule)" udp 192.168.1.53 5000 u
check6 "DN13 IPv6: UDP DNS to the tunnel resolver goes through the tunnel" udp fd77::1 53 t
check6 "DN14 IPv6: DNS to an internet resolver is blocked" udp 2001:db8:53::53 53 none
check6 "DN15 IPv6: DNS to a non-resolver through the tunnel is blocked" udp 2001:db8:99::53 53 none
check6 "DN16 IPv6: DNS to the configured LAN exception is allowed" udp 2001:db8:1::53 53 u
check6 "DN17 IPv6: DNS to another LAN DNS server is blocked" udp 2001:db8:1::54 53 none

head -1 "$RESOLV" | grep -q "Written by septuvod" && grep -q "^nameserver 10.77.0.1$" "$RESOLV" \
  && [ "$(cat "$BACKUP" 2>/dev/null)" = "$ORIGINAL" ] && ok=1 || ok=0
result $ok "DN18 the resolver file points at the tunnel resolver; the original is backed up"
inC head -1 /etc/resolv.conf | grep -q "Written by septuvod" && ok=1 || ok=0
result $ok "DN19 programs in the device namespace see Septuvo's resolver file"
err=$(inC true 2>&1)
[ -z "$err" ] && ok=1 || ok=0
result $ok "DN19b nothing else leaks into the namespace's view of /etc (no stray files)"
out=$(cli status)
grep -q "Status: Not connected" <<<"$out" && ok=1 || ok=0
result $ok "DN20 status still never claims connected"

cli disconnect > /dev/null && [ "$(cat "$RESOLV")" = "$ORIGINAL" ] && [ ! -e "$BACKUP" ] && ok=1 || ok=0
result $ok "DN21 deliberate disconnect restores the original resolver file exactly"
check "DN22 after disconnect, DNS works normally again" udp 203.0.113.53 53 u

cli connect > /dev/null
{ kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null; DPID=""
head -1 "$RESOLV" | grep -q "Written by septuvod" && ok=1 || ok=0
result $ok "DN23 after a crash the resolver file stays Septuvo's (fail closed)"
check "DN24 after a crash, DNS outside the tunnel is still blocked" udp 203.0.113.53 53 none
start_daemon >/dev/null && cli disconnect >/dev/null && [ "$(cat "$RESOLV")" = "$ORIGINAL" ] && ok=1 || ok=0
result $ok "DN25 after restart, disconnect still restores the true original"

echo
echo "dns lab: $PASS passed, $FAIL failed, $SKIP skipped (of $N; IPv6 available: $HAVE_V6)"
[ "$FAIL" = 0 ]
