#!/usr/bin/env bash
# Septuvo namespace lab (N3-02, revised after the owner's review of PR #63):
# captive-portal mode for a dedicated helper account only.
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/captive.sh [--require-ipv6] target/debug/septuvod target/debug/septuvo
#
# Namespaces (all throw-away):
#   C  the device: runs septuvod (lab mode), full tunnel, tunnel DNS
#   U  a hotel network:
#        192.168.1.1   router (and the server endpoint, UDP 5000)
#        192.168.1.66  the network's resolver, MALICIOUS: it answers
#                      portal.example -> 192.168.1.80 (the sign-in page),
#                      home.example   -> 10.77.0.1 (an address behind the tunnel)
#                      blocked.example -> 203.0.113.99 (a host the policy blocks)
#        192.168.1.80  the sign-in page (TCP 80/443)
#        192.168.1.30  a LAN device the policy lists as direct
#        192.168.1.54  another LAN DNS server
#        203.0.113.80  an internet web server
#        203.0.113.99  a host the policy blocks
#        1.1.1.1       stand-in for a trusted DNS-over-HTTPS resolver (TCP 443;
#                      plain TCP here: the lab tests the firewall path only)
#   T  the far end of a stand-in tunnel interface septuvo0 (no encryption,
#      no SVP): 10.77.0.1, the tunnel resolver and a "home" server
# Programs: "helper" runs as the dedicated captive-portal account, "root" and
# "other" (an ordinary user) are ordinary applications.
set -euo pipefail

REQUIRE_V6=0
if [ "${1:-}" = "--require-ipv6" ]; then REQUIRE_V6=1; shift; fi
DAEMON=$(realpath "${1:?usage: captive.sh [--require-ipv6] PATH/TO/septuvod PATH/TO/septuvo}")
CLI=$(realpath "${2:?usage: captive.sh [--require-ipv6] PATH/TO/septuvod PATH/TO/septuvo}")
HERE=$(cd "$(dirname "$0")" && pwd)

if [ "$(id -u)" != 0 ]; then
  echo "captive.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 runuser useradd userdel groupadd groupdel realpath getent timeout; do
  command -v "$tool" >/dev/null || { echo "captive.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-pc-$ID" U="sv-pu-$ID" T="sv-pt-$ID"
GROUP="sv-cp-$ID" MEMBER="svcm$ID" OTHER="svco$ID" HELPER="svch$ID"
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
  for u in "$MEMBER" "$OTHER" "$HELPER"; do userdel "$u" 2>/dev/null || true; done
  groupdel "$GROUP" 2>/dev/null || true
  groupdel "$HELPER" 2>/dev/null || true
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
# run WHO CMD...: WHO is helper, other or root
run() {
  local who=$1; shift
  case "$who" in
    helper) inC runuser -u "$HELPER" -- "$@" ;;
    other) inC runuser -u "$OTHER" -- "$@" ;;
    root) inC "$@" ;;
  esac
}

# --- users, binaries, namespaces ---------------------------------------------
groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
useradd -r -M -U -s /usr/sbin/nologin "$HELPER"
HELPER_UID=$(id -u "$HELPER")
mkdir "$WORK/bin"
install -m 755 "$DAEMON" "$WORK/bin/septuvod"
install -m 755 "$CLI" "$WORK/bin/septuvo"
install -m 644 "$HERE/probe.py" "$WORK/bin/probe.py"
PROBE="$WORK/bin/probe.py"

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
for host in 1 30 54 66 80; do
  ip -n "$U" addr add "192.168.1.$host/24" dev c0
done
v6 ip -n "$U" addr add 2001:db8:1::1/64 dev c0 nodad
v6 ip -n "$U" addr add 2001:db8:1::80/64 dev c0 nodad
for a in 203.0.113.80 203.0.113.99 1.1.1.1; do ip -n "$U" addr add "$a/32" dev c0; done
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

U_ADDRS="192.168.1.1 192.168.1.30 192.168.1.54 192.168.1.80 203.0.113.80 203.0.113.99 1.1.1.1"
[ $HAVE_V6 = 1 ] && U_ADDRS="$U_ADDRS 2001:db8:1::1 2001:db8:1::80"
# shellcheck disable=SC2086
ip netns exec "$U" python3 "$PROBE" listen "$WORK/u.log" "$WORK/u.ready" $U_ADDRS &
ip netns exec "$U" python3 "$PROBE" dns-server "$WORK/dns.log" "$WORK/dns.ready" 192.168.1.66 \
  portal.example=192.168.1.80 home.example=10.77.0.1 blocked.example=203.0.113.99 &
ip netns exec "$T" python3 "$PROBE" listen "$WORK/t.log" "$WORK/t.ready" 10.77.0.1 &
for _ in $(seq 50); do
  [ -f "$WORK/u.ready" ] && [ -f "$WORK/t.ready" ] && [ -f "$WORK/dns.ready" ] && break
  sleep 0.1
done
touch "$WORK/dns.log"

cat > "$WORK/septuvo.toml" <<EOF
version = 1
[server]
endpoints = ["192.168.1.1:5000"]
[routing]
mode = "full-tunnel"
direct = ["192.168.1.30/32"]
block = ["203.0.113.99/32"]
[dns]
resolvers = ["10.77.0.1"]
EOF
settings() {   # $1: extra line
  cat <<EOF
version = 1
[daemon]
mode = "lab"
lab_namespace = "$C"
runtime_dir = "$RUN"
control_group = "$GROUP"
device_config = "$WORK/septuvo.toml"
$1
EOF
}
settings "captive_helper_user = \"$HELPER\"" > "$WORK/septuvod.toml"
settings "" > "$WORK/plain.toml"
settings 'captive_helper_user = "root"' > "$WORK/root.toml"
chmod 644 "$WORK"/*.toml
mkdir -p "$RESOLV_DIR"
ORIGINAL="nameserver 192.168.1.66"
echo "$ORIGINAL" > "$RESOLV_DIR/resolv.conf"

start_daemon() {   # $1: settings file
  : > "$WORK/daemon.log"
  inC "$WORK/bin/septuvod" --settings "${1:-$WORK/septuvod.toml}" >> "$WORK/daemon.log" 2>&1 &
  DPID=$!
  for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && return 0; sleep 0.1; done
  return 1
}
kill_daemon() { { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null; DPID=""; rm -rf "$RUN"; }
cli() { local who=$1; shift; as "$who" "$WORK/bin/septuvo" "$@" --runtime-dir "$RUN"; }

# check DESCRIPTION WHO PROTO DST PORT WHERE (u, t or none)
check() {
  local desc=$1 who=$2 proto=$3 dst=$4 port=$5 where=$6 token ok
  N_TOKEN=$((${N_TOKEN:-0} + 1)); token="cp$ID-$N_TOKEN"
  run "$who" python3 "$PROBE" send "$proto" "$dst" "$port" "$token" > /dev/null
  sleep 0.2
  case "$where" in
    none) ! grep -qx "$token" "$WORK/u.log" "$WORK/t.log" 2>/dev/null && ok=1 || ok=0 ;;
    *) grep -qx "$token" "$WORK/$where.log" && ok=1 || ok=0 ;;
  esac
  result $ok "$desc"
}
check6() { if [ $HAVE_V6 = 1 ]; then check "$@"; else result skip "$1"; fi; }
# lookup WHO NAME PORT -> sets ANSWER and TOKEN; the connection's arrival is checked by the caller
lookup() {
  N_TOKEN=$((${N_TOKEN:-0} + 1)); TOKEN="cp$ID-$N_TOKEN"
  ANSWER=$(run "$1" python3 "$PROBE" resolve-connect 192.168.1.66 "$2" "$3" "$TOKEN")
  sleep 0.2
}
arrived() { grep -qx "$1" "$WORK/u.log" "$WORK/t.log" 2>/dev/null; }

# Setup is done; from here on, failing commands are expected test outcomes.
trap - ERR
set +e

# --- configuration guards -----------------------------------------------------
inC "$WORK/bin/septuvod" --settings "$WORK/root.toml" > "$WORK/g.log" 2>&1; rc=$?
[ $rc = 1 ] && ok=1 || ok=0
result $ok "CP00 the service refuses root as the captive-portal helper (exit $rc)"
start_daemon "$WORK/plain.toml" > /dev/null
cli "$MEMBER" connect > /dev/null
out=$(cli "$MEMBER" captive-portal); rc=$?
[ $rc = 1 ] && grep -q "not_configured" <<<"$out" && ! inC nft list ruleset | grep -q captive && ok=1 || ok=0
result $ok "CP01 without a helper account, captive-portal mode is unavailable and no captive rule exists"
cli "$MEMBER" disconnect > /dev/null
kill_daemon

start_daemon && ok=1 || ok=0
result $ok "CP02 septuvod starts with helper account $HELPER (uid $HELPER_UID)"
out=$(cli "$MEMBER" captive-portal); rc=$?
[ $rc = 1 ] && grep -q "only available while connected" <<<"$out" && ok=1 || ok=0
result $ok "CP03 captive-portal mode is refused while not connected (exit $rc)"
cli "$MEMBER" connect > /dev/null && ok=1 || ok=0
result $ok "CP04 connect succeeds (kill switch on)"

# --- before any window ---------------------------------------------------------
check "CP05 before: the helper cannot reach the sign-in page" helper tcp 192.168.1.80 80 none
check "CP06 before: an ordinary app cannot reach the sign-in page" root tcp 192.168.1.80 80 none
out=$(cli "$OTHER" captive-portal --minutes 1); rc=$?
[ $rc != 0 ] && ok=1 || ok=0
result $ok "CP07 a user outside the control group cannot open a window (exit $rc)"
check "CP08 ...and the helper is still blocked" helper tcp 192.168.1.80 80 none

# --- window 1: trusted DNS only (the default) -----------------------------------
out=$(cli "$MEMBER" captive-portal --minutes 1); rc=$?
[ $rc = 0 ] && grep -q "Captive portal: OPEN (" <<<"$out" && grep -q "WARNING: captive-portal window: the sign-in helper account" <<<"$out" \
  && grep -q "encrypted DNS only" <<<"$out" && grep -q "uid $HELPER_UID" <<<"$out" && ok=1 || ok=0
result $ok "CP09 a member opens a window: warning, helper account, trusted-DNS-only and countdown shown (exit $rc)"
[ $ok = 1 ] || sed 's/^/      /' <<<"$out"

check  "CP10 helper: the sign-in page on TCP 80 is reachable" helper tcp 192.168.1.80 80 u
check  "CP11 helper: an HTTPS site on TCP 443 is reachable" helper tcp 203.0.113.80 443 u
check  "CP12 helper: the trusted DoH resolver (1.1.1.1, TCP 443) is reachable" helper tcp 1.1.1.1 443 u
check  "CP13 ordinary app (root): the sign-in page stays BLOCKED" root tcp 192.168.1.80 80 none
check  "CP14 ordinary app (root): HTTPS outside the tunnel stays BLOCKED" root tcp 203.0.113.80 443 none
check  "CP15 ordinary app (other user): HTTPS outside the tunnel stays BLOCKED" other tcp 203.0.113.80 443 none
check  "CP16 ordinary app (other user): the DoH resolver outside the tunnel stays BLOCKED" other tcp 1.1.1.1 443 none
check  "CP17 helper: the network's DNS is NOT opened without explicit authorization" helper udp 192.168.1.66 53 none
check  "CP18 ordinary app: the network's DNS stays blocked" root udp 192.168.1.66 53 none
check  "CP19 helper: QUIC (UDP 443) stays blocked" helper udp 203.0.113.80 443 none
check  "CP20 helper: other TCP ports stay blocked" helper tcp 203.0.113.80 5000 none
check  "CP21 helper: DNS over TLS (853) stays blocked" helper tcp 192.168.1.54 853 none
check  "CP22 helper: never through the tunnel (TCP 80 to 10.77.0.1)" helper tcp 10.77.0.1 80 none
check  "CP23 helper: never the tunnel resolver" helper udp 10.77.0.1 53 none
check  "CP24 helper: a host the policy blocks stays blocked, even on TCP 80" helper tcp 203.0.113.99 80 none
check  "CP25 helper: a LAN device listed as direct is not reachable by the helper" helper tcp 192.168.1.30 80 none
check  "CP26 ordinary app: the direct LAN device still works as configured" root tcp 192.168.1.30 80 u
check  "CP27 helper: the server endpoint is not reachable by the helper" helper udp 192.168.1.1 5000 none
check  "CP28 ordinary app: tunnel DNS still uses the tunnel" root udp 10.77.0.1 53 t
check6 "CP29 helper: IPv6 sign-in page (TCP 80) is reachable" helper tcp 2001:db8:1::80 80 u
check6 "CP30 ordinary app: IPv6 sign-in page stays BLOCKED" root tcp 2001:db8:1::80 80 none
head -1 "$RESOLV_DIR/resolv.conf" | grep -q "Written by septuvod" && grep -q "^nameserver 10.77.0.1$" "$RESOLV_DIR/resolv.conf" && ok=1 || ok=0
result $ok "CP31 the system resolver file is unchanged during the window (tunnel resolver only)"
out=$(cli "$OTHER" status)
grep -q "Captive portal: OPEN" <<<"$out" && grep -q "WARNING:" <<<"$out" && grep -q "Status: Not connected" <<<"$out" \
  && grep -q "windows left this connection: 2" <<<"$out" && ok=1 || ok=0
result $ok "CP32 status shows the open window, the warning and windows left; never claims connected"

# --- window 2: the network's DNS, explicitly authorized; malicious answers ------
out=$(cli "$MEMBER" captive-portal --minutes 1 --network-dns); rc=$?
OPENED=$(date +%s)
[ $rc = 0 ] && grep -q "network DNS authorized for the helper only" <<<"$out" && grep -q "192.168.1.66" <<<"$out" && ok=1 || ok=0
result $ok "CP33 --network-dns is explicit, warned about and lists the network's resolver (exit $rc)"
: > "$WORK/dns.log"
lookup helper portal.example 80
grep -q "^answer:192.168.1.80 sent" <<<"$ANSWER" && arrived "$TOKEN" && ok=1 || ok=0
result $ok "CP34 helper: resolves the sign-in page through the network's DNS and reaches it ($ANSWER)"
lookup helper home.example 80
grep -q "^answer:10.77.0.1 " <<<"$ANSWER" && ! arrived "$TOKEN" && ok=1 || ok=0
result $ok "CP35 malicious answer pointing behind the tunnel cannot redirect the helper into it ($ANSWER)"
lookup helper blocked.example 80
grep -q "^answer:203.0.113.99 " <<<"$ANSWER" && ! arrived "$TOKEN" && ok=1 || ok=0
result $ok "CP36 malicious answer pointing at a policy-blocked host is still blocked ($ANSWER)"
before=$(wc -l < "$WORK/dns.log")
lookup root portal.example 80
lookup other portal.example 80
grep -q "^noanswer" <<<"$ANSWER" && [ "$(wc -l < "$WORK/dns.log")" = "$before" ] && ok=1 || ok=0
result $ok "CP37 ordinary apps cannot query the network's resolver even while it is authorized for the helper"
run root timeout 4 getent hosts portal.example > /dev/null
[ "$(wc -l < "$WORK/dns.log")" = "$before" ] && ok=1 || ok=0
result $ok "CP38 ordinary system lookups never reach the network's resolver (the system resolver is the tunnel's)"
check  "CP39 ordinary app: still blocked from the sign-in page" root tcp 192.168.1.80 80 none

# --- crash, restart and kernel expiry ------------------------------------------
kill_daemon
check  "CP40 service killed mid-window: the kernel still holds the window for the helper only" helper tcp 192.168.1.80 80 u
check  "CP41 service killed mid-window: ordinary apps stay blocked" root tcp 192.168.1.80 80 none
start_daemon > /dev/null
out=$(cli "$OTHER" status)
grep -q "time left unknown; it still ends on time" <<<"$out" && ok=1 || ok=0
result $ok "CP42 a restarted service reports the window it did not open (time left unknown)"
kill_daemon
WAIT=$((OPENED + 63 - $(date +%s)))
echo "      waiting ${WAIT}s for the kernel to end the window (service NOT running)"
[ $WAIT -gt 0 ] && sleep $WAIT
check  "CP43 after the timeout, with no service running: the helper is blocked again" helper tcp 192.168.1.80 80 none
check  "CP44 after the timeout: the helper's HTTPS is blocked again" helper tcp 203.0.113.80 443 none
check  "CP45 after the timeout: the helper's network DNS is blocked again" helper udp 192.168.1.66 53 none
check6 "CP46 after the timeout: the helper's IPv6 web access is blocked again" helper tcp 2001:db8:1::80 80 none
check  "CP47 after the timeout: ordinary apps remain blocked (protected state)" root tcp 203.0.113.80 443 none

# --- limits ----------------------------------------------------------------------
start_daemon > /dev/null
out=$(cli "$OTHER" status)
grep -q "Captive portal: closed" <<<"$out" && grep -q "Kill switch: on" <<<"$out" && ok=1 || ok=0
result $ok "CP48 status after the timeout: closed, kill switch still on"
out=$(cli "$MEMBER" captive-portal); rc=$?
[ $rc = 1 ] && grep -q "only available while connected" <<<"$out" && ok=1 || ok=0
result $ok "CP49 after a restart, rules left from the earlier run must be reconnected first (exit $rc)"
cli "$MEMBER" connect > /dev/null
cli "$MEMBER" captive-portal > /dev/null
cli "$MEMBER" captive-portal --minutes 1 > /dev/null
set_text=$(inC nft list set inet septuvo captive_web)
grep -q "80 timeout 1m" <<<"$set_text" && ! grep -q "timeout 5m" <<<"$set_text" && ! grep -q "timeout 6m" <<<"$set_text" && ok=1 || ok=0
result $ok "CP50 asking again restarts the window from now; it never adds up"
[ $ok = 1 ] || sed 's/^/      /' <<<"$set_text"
cli "$MEMBER" captive-portal --minutes 1 > /dev/null
out=$(cli "$MEMBER" captive-portal --minutes 1); rc=$?
[ $rc = 1 ] && grep -q "at most 3 captive-portal windows per connection" <<<"$out" \
  && [ "$(grep -c 'captive-portal window' "$WORK/daemon.log")" = 3 ] && ok=1 || ok=0
result $ok "CP51 a fourth window per connection is refused, and every window was logged (exit $rc)"
cli "$MEMBER" disconnect > /dev/null
! inC nft list table inet septuvo > /dev/null 2>&1 && ok=1 || ok=0
result $ok "CP52 disconnect ends the window at once (no Septuvo rules left)"
cli "$MEMBER" connect > /dev/null
check  "CP53 reconnecting starts with the window closed" helper tcp 192.168.1.80 80 none
cli "$MEMBER" disconnect > /dev/null

echo
echo "captive lab: $PASS passed, $FAIL failed, $SKIP skipped (of $N; IPv6 available: $HAVE_V6)"
[ "$FAIL" = 0 ]
