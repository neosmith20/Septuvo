#!/usr/bin/env bash
# Septuvo namespace lab (N1-03): kill switch, DNS and IPv6 leak tests.
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/run.sh [--require-ipv6] target/debug/examples/render_nft
# IPv6 cases are reported as SKIP on kernels without IPv6; --require-ipv6
# (used in CI) turns any skip into a failure.
#
# Builds three throw-away network namespaces on this machine:
#   C  the device under test (gets the generated kill switch)
#   U  the underlay: a hostile cafe network with local hosts, a local DNS
#      server, the Septuvo Server endpoint and "the internet"
#   T  the far end of a stand-in tunnel interface (septuvo0). Not a real
#      tunnel: no encryption, no SVP, just where tunnelled packets land.
# Every probe sends a unique token and checks where it arrives (T, U or
# nowhere). Nothing leaves the namespaces; nothing on the host is changed.
set -euo pipefail

REQUIRE_V6=0
if [ "${1:-}" = "--require-ipv6" ]; then REQUIRE_V6=1; shift; fi
RENDER=${1:?usage: run.sh [--require-ipv6] PATH/TO/render_nft}
RENDER=$(realpath "$RENDER")
HERE=$(cd "$(dirname "$0")" && pwd)
PROBE="$HERE/probe.py"
POLICIES="$HERE/policies"

if [ "$(id -u)" != 0 ]; then
  echo "run.sh: needs root (network namespaces); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 realpath; do
  command -v "$tool" >/dev/null || { echo "run.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-c-$ID" U="sv-u-$ID" T="sv-t-$ID"
WORK=$(mktemp -d)
PASS=0 FAIL=0 SKIP=0 N=0
HAVE_V6=0
[ -d /proc/sys/net/ipv6 ] && HAVE_V6=1
v6() { if [ $HAVE_V6 = 1 ]; then "$@"; fi; }
TUN=septuvo0
ENDPOINTS=("203.0.113.7:5000" "[2001:db8:7::7]:5000")

cleanup() {
  for ns in "$C" "$U" "$T"; do
    if ip netns list | grep -qw "$ns"; then
      ip netns pids "$ns" | xargs -r kill 2>/dev/null || true
      ip netns del "$ns" 2>/dev/null || true
    fi
  done
  rm -rf "$WORK"
}
trap cleanup EXIT

inC() { ip netns exec "$C" "$@"; }

build() {
  for ns in "$C" "$U" "$T"; do ip netns add "$ns"; ip -n "$ns" link set lo up; done
  # Underlay: the device's Wi-Fi (C eth0) <-> cafe network (U c0), 192.168.1.0/24.
  ip link add eth0 netns "$C" type veth peer name c0 netns "$U"
  ip -n "$C" addr add 192.168.1.10/24 dev eth0
  v6 ip -n "$C" addr add 2001:db8:1::10/64 dev eth0 nodad
  ip -n "$C" link set eth0 up
  ip -n "$U" addr add 192.168.1.1/24 dev c0
  v6 ip -n "$U" addr add 2001:db8:1::1/64 dev c0 nodad
  for host in 20 30 53; do   # other LAN host, approved device, local DNS
    ip -n "$U" addr add "192.168.1.$host/24" dev c0
    v6 ip -n "$U" addr add "2001:db8:1::$host/64" dev c0 nodad
  done
  ip -n "$U" link set c0 up
  # Beyond the cafe router (host addresses on U's loopback): internet, server endpoint, and a cafe device that
  # happens to use the home network's addresses (leak detectors).
  for a in 198.51.100.20/32 203.0.113.7/32 192.168.50.10/32; do ip -n "$U" addr add "$a" dev lo; done
  for a in 2001:db8:99::1/128 2001:db8:7::7/128 2001:db8:50::10/128; do v6 ip -n "$U" addr add "$a" dev lo nodad; done
  ip -n "$C" route add default via 192.168.1.1 metric 100
  v6 ip -n "$C" -6 route add default via 2001:db8:1::1 metric 100
  ip -n "$C" route add 203.0.113.7/32 via 192.168.1.1
  v6 ip -n "$C" -6 route add 2001:db8:7::7/128 via 2001:db8:1::1
  # Far end of the stand-in tunnel: the same destinations, so we can tell
  # tunnelled packets from leaked ones, plus the home network.
  for a in 198.51.100.20/32 192.168.50.10/32 192.168.1.20/32; do ip -n "$T" addr add "$a" dev lo; done
  for a in 2001:db8:99::1/128 2001:db8:50::10/128; do v6 ip -n "$T" addr add "$a" dev lo nodad; done
  # Listeners.
  for ns in "$U" "$T"; do
    ip netns exec "$ns" python3 "$PROBE" listen "$WORK/$ns.log" "$WORK/$ns.ready" &
  done
  for _ in $(seq 50); do
    [ -f "$WORK/$U.ready" ] && [ -f "$WORK/$T.ready" ] && return 0
    sleep 0.1
  done
  echo "run.sh: listeners did not start" >&2
  exit 1
}

tunnel_up() {   # $1: full | split | overlap
  ip link add "$TUN" netns "$C" type veth peer name t0 netns "$T"
  ip -n "$C" addr add 10.99.0.2/24 dev "$TUN"
  v6 ip -n "$C" addr add fd99::2/64 dev "$TUN" nodad
  ip -n "$T" addr add 10.99.0.1/24 dev t0
  v6 ip -n "$T" addr add fd99::1/64 dev t0 nodad
  ip -n "$C" link set "$TUN" up
  ip -n "$T" link set t0 up
  case "$1" in
    full)
      ip -n "$C" route add default via 10.99.0.1 dev "$TUN" metric 10
      v6 ip -n "$C" -6 route add default via fd99::1 dev "$TUN" metric 10 ;;
    split)
      ip -n "$C" route add 192.168.50.0/24 via 10.99.0.1 dev "$TUN"
      v6 ip -n "$C" -6 route add 2001:db8:50::/64 via fd99::1 dev "$TUN" ;;
    overlap)
      ip -n "$C" route add 192.168.1.0/24 via 10.99.0.1 dev "$TUN" table 100 ;;
  esac
}

tunnel_down() { ip -n "$C" link del "$TUN" 2>/dev/null || true; }

apply_policy() {   # $1: policy file name
  "$RENDER" "$POLICIES/$1" "$TUN" "${ENDPOINTS[@]}" > "$WORK/rules.nft"
  inC nft -f "$WORK/rules.nft"
}

result() {   # $1: ok (0/1), $2: description
  if [ "$1" = 1 ]; then PASS=$((PASS + 1)); echo "PASS  $2"; else FAIL=$((FAIL + 1)); echo "FAIL  $2"; fi
}

# check NAME PROTO DST PORT EXPECT [SPORT]; EXPECT = tunnel | underlay | blocked
check() {
  local name=$1 proto=$2 dst=$3 port=$4 expect=$5 sport=${6:-}
  N=$((N + 1))
  if [ $HAVE_V6 = 0 ] && [[ "$dst" == *:* ]]; then
    SKIP=$((SKIP + 1)); echo "SKIP  $name: kernel has no IPv6 (not run, not passed)"; return 0
  fi
  local token="sv-token-$ID-$N" sent in_t=0 in_u=0 ok=0
  sent=$(inC python3 "$PROBE" send "$proto" "$dst" "$port" "$token" $sport)
  sleep 0.3
  grep -qx "$token" "$WORK/$T.log" 2>/dev/null && in_t=1
  grep -qx "$token" "$WORK/$U.log" 2>/dev/null && in_u=1
  case "$expect" in
    tunnel)   [ $in_t = 1 ] && [ $in_u = 0 ] && ok=1 ;;
    underlay) [ $in_u = 1 ] && [ $in_t = 0 ] && ok=1 ;;
    blocked)  [ $in_t = 0 ] && [ $in_u = 0 ] && ok=1 ;;
  esac
  local where=nowhere
  [ $in_t = 1 ] && where=tunnel
  [ $in_u = 1 ] && where=underlay
  [ $in_t = 1 ] && [ $in_u = 1 ] && where=both
  result $ok "$name: $proto $dst:$port expected $expect, arrived $where (sender: $sent)"
}

build
echo "== Controls: no kill switch installed, so these packets must leak (proves the detector works)"
check "C01 without kill switch, internet leaks to cafe"   udp 198.51.100.20   5000  underlay
check "C02 without kill switch, IPv6 leaks to cafe"       udp 2001:db8:99::1  5000  underlay
check "C03 without kill switch, DNS goes to cafe resolver" udp 192.168.1.53   53    underlay
tunnel_up overlap
check "C04 without kill switch, home device goes to cafe" udp 192.168.1.20    5000  underlay
tunnel_down

echo "== Journey B: untrusted Wi-Fi, full tunnel, one approved local device (NP-S1, NP-S4)"
apply_policy journey-b.policy
tunnel_up full
check "B01 internet IPv4 through tunnel"          udp 198.51.100.20   5000  tunnel
check "B02 internet IPv6 through tunnel"          udp 2001:db8:99::1  5000  tunnel
check "B03 DNS to tunnel resolver"                udp 10.99.0.1       53    tunnel
check "B04 DNS to cafe resolver blocked (NP-D1)"  udp 192.168.1.53    53    blocked
check "B05 DNS-over-TLS to cafe blocked"          tcp 192.168.1.53    853   blocked
check "B06 IPv6 DNS to cafe resolver blocked"     udp 2001:db8:1::53  53    blocked
check "B07 approved device IPv4 direct"           udp 192.168.1.30    5000  underlay
check "B08 approved device IPv6 direct (ND ok)"   udp 2001:db8:1::30  5000  underlay
check "B09 DNS to approved device blocked"        udp 192.168.1.30    53    blocked
check "B10 other LAN host blocked (no RFC1918 trust)" udp 192.168.1.20 5000 blocked
check "B11 server endpoint IPv4"                  udp 203.0.113.7     5000  underlay
check "B12 server endpoint IPv6"                  udp 2001:db8:7::7   5000  underlay
check "B13 server endpoint, other port blocked"   udp 203.0.113.7     53    blocked
check "B14 DHCPv4 to cafe router"                 udp 192.168.1.1     67    underlay 68
check "B15 DHCPv6 to cafe router"                 udp 2001:db8:1::1   547   underlay 546
# Hostile network pushes a more specific route around the tunnel (for example
# DHCP option 121, "TunnelVision"): the firewall must still stop it (NP-E1).
inC ip route add 198.51.100.0/24 via 192.168.1.1
v6 inC ip -6 route add 2001:db8:99::/64 via 2001:db8:1::1
check "B16 hostile IPv4 route cannot leak"        udp 198.51.100.20   5000  blocked
check "B17 hostile IPv6 route cannot leak"        udp 2001:db8:99::1  5000  blocked
inC ip route del 198.51.100.0/24 via 192.168.1.1
v6 inC ip -6 route del 2001:db8:99::/64 via 2001:db8:1::1
tunnel_down
check "B20 kill switch IPv4 (path down)"          udp 198.51.100.20   5000  blocked
check "B21 kill switch IPv6 (path down)"          udp 2001:db8:99::1  5000  blocked
check "B22 kill switch TCP (path down)"           tcp 198.51.100.20   5000  blocked
check "B23 tunnel DNS unreachable, no fallback"   udp 10.99.0.1       53    blocked
check "B24 cafe DNS still blocked (path down)"    udp 192.168.1.53    53    blocked
check "B25 approved device still direct"          udp 192.168.1.30    5000  underlay
check "B26 server endpoint still reachable"       udp 203.0.113.7     5000  underlay
check "B27 DHCP still works"                      udp 192.168.1.1     67    underlay 68
check "B28 other LAN host still blocked"          udp 192.168.1.20    5000  blocked

echo "== Journey A: home network from the cafe, split tunnel (NP-S2)"
apply_policy journey-a.policy
tunnel_up split
check "A01 home NAS IPv4 through tunnel"          udp 192.168.50.10   5000  tunnel
check "A02 home NAS IPv6 through tunnel"          udp 2001:db8:50::10 5000  tunnel
check "A03 internet direct (split: unprotected)"  udp 198.51.100.20   5000  underlay
check "A04 cafe DNS allowed in split mode"        udp 192.168.1.53    53    underlay
tunnel_down
check "A10 home NAS IPv4 fails closed"            udp 192.168.50.10   5000  blocked
check "A11 home NAS IPv6 fails closed"            udp 2001:db8:50::10 5000  blocked
check "A12 internet still direct"                 udp 198.51.100.20   5000  underlay

echo "== Overlapping subnets: home and cafe both 192.168.1.0/24 (NP-S5)"
apply_policy overlap.policy
inC ip rule add to 192.168.1.1/32 lookup main pref 50
inC ip rule add to 192.168.1.0/24 lookup 100 pref 100
tunnel_up overlap
check "O01 home device through tunnel"            udp 192.168.1.20    5000  tunnel
check "O02 DHCP to cafe router still works"       udp 192.168.1.1     67    underlay 68
check "O03 server endpoint still reachable"       udp 203.0.113.7     5000  underlay
tunnel_down
check "O10 home device never sent to cafe"        udp 192.168.1.20    5000  blocked
check "O11 DHCP still works"                      udp 192.168.1.1     67    underlay 68
inC ip rule del pref 100
inC ip rule del pref 50

echo "== Apply, replace and roll back (NP-E6)"
apply_policy journey-b.policy
before=$(inC nft list ruleset)
{ cat "$WORK/rules.nft"; echo "this line is not valid nftables"; } > "$WORK/broken.nft"
if inC nft -f "$WORK/broken.nft" 2>/dev/null; then ok=0; else ok=1; fi
[ "$before" = "$(inC nft list ruleset)" ] || ok=0
result $ok "R01 invalid ruleset rejected as a whole; previous rules unchanged"
apply_policy journey-a.policy
tables=$(inC nft list tables | grep -c septuvo || true)
if [ "$tables" = 1 ] && ! inC nft list ruleset | grep -q 192.168.1.30; then ok=1; else ok=0; fi
result $ok "R02 applying a new policy replaces the old one completely"
"$RENDER" --remove > "$WORK/remove.nft"
inC nft -f "$WORK/remove.nft"
if [ -z "$(inC nft list ruleset)" ]; then ok=1; else ok=0; fi
result $ok "R03 removal leaves no rules behind"
if inC nft -f "$WORK/remove.nft"; then ok=1; else ok=0; fi
result $ok "R04 removal is safe to repeat"
check "R05 normal networking restored after removal" udp 198.51.100.20 5000 underlay

echo
echo "Summary: $PASS passed, $FAIL failed, $SKIP skipped (IPv6 available: $HAVE_V6)"
if [ "$REQUIRE_V6" = 1 ] && [ "$SKIP" != 0 ]; then
  echo "FAIL  IPv6 cases were skipped but --require-ipv6 was given"
  exit 1
fi
[ "$FAIL" = 0 ]
