#!/usr/bin/env bash
# Septuvo namespace lab (N4-04): Local Network Access (lab case T2-LAN-01,
# D-040, NP-L1 to NP-L7).
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/lan.sh [--require-ipv6] target/debug/septuvod target/debug/septuvo
#
# Three throw-away namespaces: C (the device), U (its upstream network with
# the server endpoint 10.200.0.1) and L (a local network with a device at
# 192.168.50.2). The device's own addresses decide what Local Network Access
# may open, so the lab gives C hostile ones, as fake DHCP or router
# advertisements would: a /8, a /1 and an IPv6 /32 (too large), a subnet
# that overlaps a blocked prefix of the connection, and the endpoint's own
# subnet. Only the ordinary /24 (and /64) may open; discovery (mDNS, LLMNR,
# SSDP, broadcast, multicast) stays blocked; address changes are followed.
# Nothing leaves the namespaces; everything is removed afterwards.
set -euo pipefail

REQUIRE_V6=0
if [ "${1:-}" = "--require-ipv6" ]; then REQUIRE_V6=1; shift; fi
usage="usage: lan.sh [--require-ipv6] PATH/TO/septuvod PATH/TO/septuvo"
DAEMON=$(realpath "${1:?$usage}")
CLI=$(realpath "${2:?$usage}")
HERE=$(cd "$(dirname "$0")" && pwd)

if [ "$(id -u)" != 0 ]; then
  echo "lan.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 runuser useradd userdel groupadd groupdel realpath; do
  command -v "$tool" >/dev/null || { echo "lan.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-nc-$ID" U="sv-nu-$ID" L="sv-nl-$ID"
GROUP="sv-ng-$ID" MEMBER="svnm$ID" OTHER="svno$ID"
WORK=$(mktemp -d)
chmod 755 "$WORK"
RUN="$WORK/run"
STORE="$WORK/store/connections"
PASS=0 FAIL=0 SKIP=0 N=0
HAVE_V6=0
[ -d /proc/sys/net/ipv6 ] && HAVE_V6=1
DPID="" LPID="" UPID=""

cleanup() {
  for p in "$LPID" "$UPID"; do [ -n "$p" ] && kill "$p" 2>/dev/null || true; done
  [ -n "$DPID" ] && { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null || true
  for ns in "$C" "$U" "$L"; do
    if ip netns list | grep -qw "$ns"; then
      ip netns pids "$ns" | xargs -r kill -9 2>/dev/null || true
      ip netns del "$ns" 2>/dev/null || true
    fi
  done
  userdel "$MEMBER" 2>/dev/null || true
  userdel "$OTHER" 2>/dev/null || true
  groupdel "$GROUP" 2>/dev/null || true
  rm -rf "/etc/netns/$C" "/etc/netns/.septuvo-$C.resolv.conf.backup" "/etc/netns/.septuvo-$C.resolv.conf.tmp" "$WORK"
}
trap cleanup EXIT
trap 'echo "::error title=lan lab setup::lan.sh line $LINENO failed: $BASH_COMMAND"' ERR

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
      if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=lan lab::$2"; fi
      if [ -f "$WORK/daemon.log" ]; then sed 's/^/      daemon: /' "$WORK/daemon.log" | tail -5; fi ;;
  esac
}

inC() { ip netns exec "$C" "$@"; }
sv() { runuser -u "$MEMBER" -- "$WORK/bin/septuvo" "$@" --runtime-dir "$RUN"; }

groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
mkdir "$WORK/bin" "$WORK/store"
chmod 700 "$WORK/store"
install -m 755 "$DAEMON" "$WORK/bin/septuvod"
install -m 755 "$CLI" "$WORK/bin/septuvo"

for ns in "$C" "$U" "$L"; do
  ip netns add "$ns"
  ip -n "$ns" link set lo up
done
# Upstream: the server endpoint 10.200.0.1 sits in the device's own subnet.
ip link add eth0 netns "$C" type veth peer name c0 netns "$U"
ip -n "$C" addr add 10.200.0.10/24 dev eth0
ip -n "$U" addr add 10.200.0.1/24 dev c0
# The local network, with the hostile prefixes a fake DHCP server or router
# advertisement could hand out.
ip link add eth1 netns "$C" type veth peer name c1 netns "$L"
for a in 192.168.50.10/24 192.168.60.10/24 10.99.0.10/8 1.2.3.4/1; do ip -n "$C" addr add "$a" dev eth1; done
for a in 192.168.50.2/24 192.168.60.2/24 192.168.70.2/24 10.99.0.2/8 100.1.1.1/1; do ip -n "$L" addr add "$a" dev c1; done
LLISTEN=(192.168.50.2 192.168.60.2 192.168.70.2 10.99.0.2 100.1.1.1)
if [ $HAVE_V6 = 1 ]; then
  ip -n "$C" addr add 2001:db8:50::10/64 dev eth1 nodad
  ip -n "$C" addr add 2001:db8:77::10/32 dev eth1 nodad
  ip -n "$L" addr add 2001:db8:50::2/64 dev c1 nodad
  ip -n "$L" addr add 2001:db8:99::2/32 dev c1 nodad
  LLISTEN+=(2001:db8:50::2 2001:db8:99::2)
fi
for l in "$C eth0" "$C eth1" "$U c0" "$L c1"; do set -- $l; ip -n "$1" link set "$2" up; done
ip -n "$C" route add default via 10.200.0.1

ip netns exec "$U" python3 "$HERE/probe.py" listen "$WORK/u.log" "$WORK/u.ready" 10.200.0.1 &
UPID=$!
ip netns exec "$L" python3 "$HERE/probe.py" listen "$WORK/l.log" "$WORK/l.ready" "${LLISTEN[@]}" &
LPID=$!
for _ in $(seq 50); do [ -f "$WORK/u.ready" ] && [ -f "$WORK/l.ready" ] && break; sleep 0.1; done

# Full tunnel through server 10.200.0.1:5000; 192.168.60.0/24 is blocked by
# the connection's policy, so the tunnel's rules win over the local network.
cat > "$WORK/septuvo.toml" <<'EOF'
version = 1
[server]
endpoints = ["10.200.0.1:5000"]
[routing]
mode = "full-tunnel"
block = ["192.168.60.0/24"]
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
connections_dir = "$STORE"
EOF
chmod 644 "$WORK"/*.toml

: > "$WORK/daemon.log"
inC "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
DPID=$!
send() { inC python3 "$HERE/probe.py" send "$@"; }
reaches() {   # PROTO DST PORT TOKEN: sent and arrived on the local network
  [ "$(send "$1" "$2" "$3" "$4")" = sent ] && sleep 0.3 && grep -qx "$4" "$WORK/l.log"
}
stopped() {   # PROTO DST PORT TOKEN: stopped by the firewall, nothing arrived
  # UDP: the kernel refuses with EPERM at once. TCP: the dropped SYN times
  # out (a reset, ECONNREFUSED, would prove the packet got out).
  local r want=refused:EPERM; r=$(send "$1" "$2" "$3" "$4")
  [ "$1" = tcp ] && want=refused:timeout
  sleep 0.2
  [ "$r" = "$want" ] || echo "      probe said: $r ($*)"
  [ "$r" = "$want" ] && ! grep -qx "$4" "$WORK/l.log" "$WORK/u.log" 2>/dev/null
}
open_list() { sv status | sed -n 's/^Local networks open: //p'; }

trap - ERR
set +e

ok=0
for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && { ok=1; break; }; sleep 0.1; done
result $ok "N00 septuvod starts"
sv connect >/dev/null 2>&1
st=$(sv status)
grep -q "Local Network Access: not-allowed" <<<"$st" && stopped udp 192.168.50.2 5000 tok-na && ok=1 || ok=0
result $ok "N01 Local Network Access is Not Allowed by default: the local device is unreachable"
runuser -u "$OTHER" -- "$WORK/bin/septuvo" local-network allowed --runtime-dir "$RUN" >/dev/null 2>&1 && ok=0 || ok=1
result $ok "N02 a user outside the control group cannot allow it"

out=$(sv local-network allowed 2>&1); rc=$?
[ $rc = 0 ] && grep -q "outside the VPN" <<<"$out" && grep -q "Local Network Access: allowed" <<<"$(sv status)" && ok=1 || ok=0
result $ok "N03 a member allows it; the service confirms and says devices can reach each other outside the VPN"
list=$(open_list)
want="192.168.50.0/24"
[ $HAVE_V6 = 1 ] && want="2001:db8:50::/64, 192.168.50.0/24, fe80::/10"
[ "$list" = "$want" ] && ok=1 || ok=0
result $ok "N04 exactly the ordinary attached subnet is opened: '$list' (expected '$want')"
reaches udp 192.168.50.2 5000 tok-lan && reaches tcp 192.168.50.2 80 tok-lan-tcp && ok=1 || ok=0
result $ok "N05 the local device is reachable (UDP and TCP)"

stopped udp 10.99.0.2 5000 tok-8 && stopped udp 100.1.1.1 5000 tok-1 && ok=1 || ok=0
result $ok "T2-LAN-01a hostile DHCP-sized prefixes (/8, /1) open nothing"
if [ $HAVE_V6 = 1 ]; then
  stopped udp 2001:db8:99::2 5000 tok-v632 && reaches udp 2001:db8:50::2 5000 tok-v664 && ok=1 || ok=0
  result $ok "T2-LAN-01b a hostile router-advertisement-sized IPv6 /32 opens nothing; the /64 does"
else
  result skip "T2-LAN-01b a hostile router-advertisement-sized IPv6 /32 opens nothing; the /64 does"
fi
stopped udp 192.168.60.2 5000 tok-ov && ok=1 || ok=0
result $ok "T2-LAN-01c a subnet overlapping a blocked prefix of the connection is not opened (the tunnel wins)"
stopped tcp 10.200.0.1 80 tok-ep && ok=1 || ok=0
result $ok "T2-LAN-01d the subnet containing the server endpoint is not opened"

disc=1
for d in "udp 224.0.0.251 5353 d-mdns" "udp 224.0.0.252 5355 d-llmnr" "udp 239.255.255.250 1900 d-ssdp" \
         "udp 192.168.50.255 5000 d-bcast" "udp 255.255.255.255 5000 d-allbcast" "udp 192.168.50.2 5353 d-umdns" \
         "udp 192.168.50.2 1900 d-ussdp" "udp 192.168.50.2 5355 d-ullmnr" "tcp 192.168.50.2 5355 d-tllmnr"; do
  set -- $d
  stopped "$@" || { disc=0; echo "      not blocked: $d"; }
done
if [ $HAVE_V6 = 1 ]; then
  stopped udp ff02::fb 5353 d-mdns6 || { disc=0; echo "      not blocked: ff02::fb 5353"; }
fi
result $disc "T2-LAN-01e discovery stays blocked: mDNS, LLMNR, SSDP (multicast and to the local device), broadcast"

# Address change (NP-L6): a new ordinary subnet opens, a removed one closes.
ip -n "$C" addr add 192.168.70.10/24 dev eth1
ip -n "$C" addr del 192.168.50.10/24 dev eth1
ok=0
for _ in $(seq 30); do
  list=$(open_list)
  if grep -q "192.168.70.0/24" <<<"$list" && ! grep -q "192.168.50.0/24" <<<"$list"; then ok=1; break; fi
  sleep 0.5
done
[ $ok = 1 ] && reaches udp 192.168.70.2 5000 tok-new && stopped udp 192.168.50.2 5000 tok-old || ok=0
result $ok "N06 an address change is followed: the new subnet opens, the old one closes ('$list')"
ip -n "$C" addr add 192.168.50.10/24 dev eth1
for _ in $(seq 30); do grep -q "192.168.50.0/24" <<<"$(open_list)" && break; sleep 0.5; done

# In Lockdown Mode the same subnets stay reachable; nothing else does.
sv lockdown on >/dev/null 2>&1
sv disconnect >/dev/null 2>&1
grep -q "Service state: lockdown" <<<"$(sv status)" && reaches udp 192.168.50.2 5000 tok-ld \
  && stopped udp 10.99.0.2 5000 tok-ld8 && stopped udp 192.168.60.2 5000 tok-ldov \
  && stopped udp 224.0.0.251 5353 tok-ldmdns && ok=1 || ok=0
result $ok "N07 in Lockdown Mode the allowed local subnet stays reachable; hostile, overlapping and discovery traffic stays blocked"

out=$(sv local-network not-allowed 2>&1)
grep -q "Local Network Access: not-allowed" <<<"$(sv status)" && [ -z "$(open_list)" ] \
  && stopped udp 192.168.50.2 5000 tok-off && ok=1 || ok=0
result $ok "N08 Not Allowed again: every local subnet is closed at once"
sv lockdown off >/dev/null 2>&1

{ kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null; DPID=""
set -e

echo
echo "lan lab: $PASS passed, $FAIL failed, $SKIP skipped (of $N)"
[ "$FAIL" = 0 ]
