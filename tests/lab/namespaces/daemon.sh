#!/usr/bin/env bash
# Septuvo namespace lab (N2-02): the privileged service septuvod and the
# unprivileged septuvo command.
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/daemon.sh target/debug/septuvod target/debug/septuvo
#
# Builds two throw-away network namespaces (C: the device, U: its network),
# two throw-away users (one in the control group, one not) and a temporary
# runtime directory. Checks the safety guards, socket permissions, connect
# and disconnect, that the kill switch really blocks traffic, that status
# never claims "connected", and that a crash keeps blocking (owner decision
# Q2). Nothing leaves the namespaces; everything is removed afterwards.
set -euo pipefail

DAEMON=$(realpath "${1:?usage: daemon.sh PATH/TO/septuvod PATH/TO/septuvo}")
CLI=$(realpath "${2:?usage: daemon.sh PATH/TO/septuvod PATH/TO/septuvo}")
HERE=$(cd "$(dirname "$0")" && pwd)

if [ "$(id -u)" != 0 ]; then
  echo "daemon.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 runuser useradd userdel groupadd groupdel realpath; do
  command -v "$tool" >/dev/null || { echo "daemon.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-dc-$ID" U="sv-du-$ID"
GROUP="sv-lab-$ID" MEMBER="svm$ID" OTHER="svo$ID"
WORK=$(mktemp -d)
chmod 755 "$WORK"
RUN="$WORK/run"
PASS=0 FAIL=0 N=0
DPID=""

cleanup() {
  [ -n "$DPID" ] && { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null || true
  for ns in "$C" "$U"; do
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
trap 'echo "::error title=daemon lab setup::daemon.sh line $LINENO failed: $BASH_COMMAND"' ERR

result() {   # $1: 1 = pass, 0 = fail; $2: description
  N=$((N + 1))
  if [ "$1" = 1 ]; then
    PASS=$((PASS + 1)); echo "PASS  $2"
  else
    FAIL=$((FAIL + 1)); echo "FAIL  $2"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=daemon lab::$2"; fi
    if [ -f "$WORK/daemon.log" ]; then sed 's/^/      daemon: /' "$WORK/daemon.log" | tail -5; fi
  fi
}

inC() { ip netns exec "$C" "$@"; }
as() { local who=$1; shift; runuser -u "$who" -- "$@"; }
table_present() { inC nft list table inet septuvo >/dev/null 2>&1; }

# --- users, binaries, namespaces -------------------------------------------
groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
mkdir "$WORK/bin"
install -m 755 "$DAEMON" "$WORK/bin/septuvod"
install -m 755 "$CLI" "$WORK/bin/septuvo"

for ns in "$C" "$U"; do
  ip netns add "$ns"
  ip -n "$ns" link set lo up
done
ip link add eth0 netns "$C" type veth peer name c0 netns "$U"
ip -n "$C" addr add 10.200.0.10/24 dev eth0
ip -n "$C" link set eth0 up
ip -n "$U" addr add 10.200.0.1/24 dev c0
ip -n "$U" link set c0 up
ip -n "$C" route add default via 10.200.0.1

# Listener on the network side: records every token it receives.
ip netns exec "$U" python3 "$HERE/probe.py" listen "$WORK/u.log" "$WORK/u.ready" 10.200.0.1 &
for _ in $(seq 50); do [ -f "$WORK/u.ready" ] && break; sleep 0.1; done

cat > "$WORK/septuvo.toml" <<'EOF'
version = 1
[server]
endpoints = ["10.200.0.1:5000"]
[routing]
mode = "full-tunnel"
[dns]
resolvers = ["10.77.0.1"]
EOF
settings() {   # $1: namespace name, $2: mode
  cat <<EOF
version = 1
[daemon]
mode = "$2"
lab_namespace = "$1"
runtime_dir = "$RUN"
control_group = "$GROUP"
device_config = "$WORK/septuvo.toml"
connections_dir = "$WORK/connections"
EOF
}
settings "$C" lab > "$WORK/septuvod.toml"
chmod 644 "$WORK"/*.toml

send() {   # $1: port, $2: token; prints "sent" or "refused:..."
  inC python3 "$HERE/probe.py" send udp 10.200.0.1 "$1" "$2"
}
arrived() { sleep 0.3; grep -qx "$1" "$WORK/u.log" 2>/dev/null; }

start_daemon() {
  : > "$WORK/daemon.log"
  inC "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
  DPID=$!
  for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && return 0; sleep 0.1; done
  return 1
}

# --- safety guards -----------------------------------------------------------
# Setup is done; from here on, failing commands are expected test outcomes.
trap - ERR
set +e
"$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" > "$WORK/g1.log" 2>&1; rc=$?
[ $rc = 3 ] && grep -q "refusing to start" "$WORK/g1.log" && ok=1 || ok=0
result $ok "D01 septuvod refuses to run in the host network namespace (exit $rc)"

settings "$C" production > "$WORK/prod.toml"
inC "$WORK/bin/septuvod" --settings "$WORK/prod.toml" > "$WORK/g2.log" 2>&1; rc=$?
[ $rc = 1 ] && grep -q 'only "lab"' "$WORK/g2.log" && ok=1 || ok=0
result $ok "D02 septuvod refuses any mode other than lab (exit $rc)"

ip netns exec "$U" "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" > "$WORK/g3.log" 2>&1; rc=$?
[ $rc = 3 ] && ok=1 || ok=0
result $ok "D03 septuvod refuses to run in a namespace other than the one named (exit $rc)"
[ ! -e "$RUN" ] && ok=1 || ok=0
result $ok "D04 refused starts create no sockets"
set -e

# --- service running ---------------------------------------------------------
start_daemon && ok=1 || ok=0
result $ok "D05 septuvod starts in the lab namespace"

set +e
out=$(as "$OTHER" "$WORK/bin/septuvo" status --runtime-dir "$RUN"); rc=$?
[ $rc = 3 ] && grep -q "Server identity verified   failed" <<<"$out" && grep -q "Kill switch: off" <<<"$out" && ok=1 || ok=0
result $ok "D06 any local user can read status; it is not connected and the kill switch is off"

out=$(as "$OTHER" "$WORK/bin/septuvo" connect --runtime-dir "$RUN"); rc=$?
[ $rc = 1 ] && grep -qi "permission denied" <<<"$out" && ! table_present && ok=1 || ok=0
result $ok "D07 a user outside the control group cannot connect (exit $rc) and nothing changes"
as "$OTHER" ls "$RUN/control" >/dev/null 2>&1 && ok=0 || ok=1
result $ok "D08 a user outside the control group cannot even list the control directory"

[ "$(send 53 tok-before)" = sent ] && arrived tok-before && ok=1 || ok=0
result $ok "D09 before connect, traffic flows normally (no Septuvo rules)"

out=$(as "$MEMBER" "$WORK/bin/septuvo" connect --runtime-dir "$RUN"); rc=$?
[ $rc = 0 ] && grep -q "Kill switch: on" <<<"$out" && table_present && ok=1 || ok=0
result $ok "D10 a control-group member can connect: the kill switch is installed (exit $rc)"

r=$(send 53 tok-blocked)
[ "$r" != sent ] && ! arrived tok-blocked && ok=1 || ok=0
result $ok "D11 with the kill switch on, traffic outside the tunnel is blocked ($r)"
[ "$(send 5000 tok-endpoint)" = sent ] && arrived tok-endpoint && ok=1 || ok=0
result $ok "D12 the configured server endpoint is still reachable"

out=$(as "$OTHER" "$WORK/bin/septuvo" status --runtime-dir "$RUN"); rc=$?
[ $rc = 3 ] && grep -q "Status: Not connected" <<<"$out" && grep -q "Service state: blocking" <<<"$out" && ok=1 || ok=0
result $ok "D13 status never claims connected (no peer before G1), shows blocking"

python3 - "$RUN/status.sock" <<'PY'
import socket, sys
for payload in (b"x" * 5000, b"SEPTUVO/1 CONNECT\n", b"\xff\xfe\n", b""):
    s = socket.socket(socket.AF_UNIX); s.settimeout(3); s.connect(sys.argv[1])
    try:
        s.sendall(payload); s.shutdown(socket.SHUT_WR); s.recv(4096)
    except OSError:
        pass
    s.close()
PY
kill -0 "$DPID" 2>/dev/null && table_present && ok=1 || ok=0
result $ok "D14 hostile requests on the status socket change nothing and do not stop the service"

out=$(as "$MEMBER" "$WORK/bin/septuvo" disconnect --runtime-dir "$RUN"); rc=$?
[ $rc = 0 ] && ! table_present && [ "$(send 53 tok-after)" = sent ] && arrived tok-after && ok=1 || ok=0
result $ok "D15 deliberate disconnect removes the rules and traffic flows again (exit $rc)"

as "$MEMBER" "$WORK/bin/septuvo" connect --runtime-dir "$RUN" >/dev/null
{ kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null; DPID=""
table_present && [ "$(send 53 tok-crash)" != sent ] && ok=1 || ok=0
result $ok "D16 if the service crashes, the kill switch stays on (fail closed)"

start_daemon && out=$(as "$OTHER" "$WORK/bin/septuvo" status --runtime-dir "$RUN")
grep -q "blocking-left-from-earlier-run" <<<"$out" && table_present && ok=1 || ok=0
result $ok "D17 after a restart the old rules are kept and reported, not silently removed"
set -e

echo
echo "daemon lab: $PASS passed, $FAIL failed (of $N)"
[ "$FAIL" = 0 ]
