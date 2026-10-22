#!/usr/bin/env bash
# Septuvo namespace lab (N4-03): saved connections and switching while
# connected (lab case T2-SW-01, D-041).
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/switch.sh target/debug/septuvod target/debug/septuvo
#
# Two throw-away namespaces (C: the device, U: its network), two throw-away
# users and a private connection store under a temporary directory.
# Connection A allows only UDP 5000 to the network side, connection B only
# UDP 443; port 53 is outside both. While traffic floods out of C on all
# three ports, the service switches between A and B many times. No packet
# outside the policy may get out, and the rules must never be missing. A
# bad connection must change nothing. Nothing leaves the namespaces;
# everything is removed afterwards.
set -euo pipefail

DAEMON=$(realpath "${1:?usage: switch.sh PATH/TO/septuvod PATH/TO/septuvo}")
CLI=$(realpath "${2:?usage: switch.sh PATH/TO/septuvod PATH/TO/septuvo}")
HERE=$(cd "$(dirname "$0")" && pwd)

if [ "$(id -u)" != 0 ]; then
  echo "switch.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 runuser useradd userdel groupadd groupdel realpath; do
  command -v "$tool" >/dev/null || { echo "switch.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-sc-$ID" U="sv-su-$ID"
GROUP="sv-sg-$ID" MEMBER="svsm$ID" OTHER="svso$ID"
WORK=$(mktemp -d)
chmod 755 "$WORK"
RUN="$WORK/run"
STORE="$WORK/store/connections"
PASS=0 FAIL=0 N=0
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
  groupdel "$GROUP" 2>/dev/null || true
  rm -rf "/etc/netns/$C" "/etc/netns/.septuvo-$C.resolv.conf.backup" "/etc/netns/.septuvo-$C.resolv.conf.tmp" "$WORK"
}
trap cleanup EXIT
trap 'echo "::error title=switch lab setup::switch.sh line $LINENO failed: $BASH_COMMAND"' ERR

result() {   # $1: 1 = pass, 0 = fail; $2: description
  N=$((N + 1))
  if [ "$1" = 1 ]; then
    PASS=$((PASS + 1)); echo "PASS  $2"
  else
    FAIL=$((FAIL + 1)); echo "FAIL  $2"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=switch lab::$2"; fi
    if [ -f "$WORK/daemon.log" ]; then sed 's/^/      daemon: /' "$WORK/daemon.log" | tail -5; fi
  fi
}

inC() { ip netns exec "$C" "$@"; }
sv() { runuser -u "$MEMBER" -- "$WORK/bin/septuvo" "$@" --runtime-dir "$RUN"; }
table() { inC nft list table inet septuvo 2>/dev/null; }

groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
mkdir "$WORK/bin" "$WORK/store" "$WORK/out"
chmod 700 "$WORK/store"
chown "$MEMBER" "$WORK/out"
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

ip netns exec "$U" python3 "$HERE/probe.py" listen "$WORK/u.log" "$WORK/u.ready" 10.200.0.1 &
LPID=$!
for _ in $(seq 50); do [ -f "$WORK/u.ready" ] && break; sleep 0.1; done

# The settings' own configuration (used when nothing is selected) and two
# saved connections that differ only in which server port they allow.
conn() {   # $1: port
  cat <<EOF
version = 1
[server]
endpoints = ["10.200.0.1:$1"]
[routing]
mode = "full-tunnel"
[dns]
resolvers = ["10.77.0.1"]
EOF
}
conn 6000 > "$WORK/septuvo.toml"
conn 5000 > "$WORK/a.toml"
conn 443 > "$WORK/b.toml"
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

start_daemon() {
  : > "$WORK/daemon.log"
  inC "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
  DPID=$!
  for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && return 0; sleep 0.1; done
  return 1
}
send() { inC python3 "$HERE/probe.py" send udp 10.200.0.1 "$1" "$2"; }
arrived() { sleep 0.3; grep -qx "$1" "$WORK/u.log" 2>/dev/null; }

trap - ERR
set +e

start_daemon && ok=1 || ok=0
result $ok "S00 septuvod starts with an empty private store"
[ "$(stat -c '%a %u' "$STORE")" = "700 0" ] && ok=1 || ok=0
result $ok "S01 the store directory is root-owned and private (mode 700)"

out=$(sv connection add "$WORK/a.toml" --name "Home – Büro A" 2>&1)
grep -q "Saved as c0001" <<<"$out" && ok=1 || ok=0
result $ok "S02 a member can save a connection with a Unicode name ($out)"
out=$(sv connection add "$WORK/b.toml" --name "Office B" 2>&1)
grep -q "Saved as c0002" <<<"$out" && ok=1 || ok=0
result $ok "S03 a second connection is saved"
list=$(runuser -u "$OTHER" -- "$WORK/bin/septuvo" connection list --runtime-dir "$RUN")
grep -q "c0001  Home – Büro A" <<<"$list" && grep -q "c0002  Office B" <<<"$list" && ok=1 || ok=0
result $ok "S04 any local user can list the names"
runuser -u "$OTHER" -- "$WORK/bin/septuvo" connection show c0001 --runtime-dir "$RUN" >/dev/null 2>&1 && ok=0 || ok=1
result $ok "S05 a user outside the control group cannot read a configuration"
runuser -u "$OTHER" -- "$WORK/bin/septuvo" connection add "$WORK/a.toml" --name X --runtime-dir "$RUN" >/dev/null 2>&1 && ok=0 || ok=1
result $ok "S06 a user outside the control group cannot add one"

sv connection add "$WORK/a.toml" --name "home – büro a" >/dev/null 2>&1 && ok=0 || ok=1
result $ok "S07 a duplicate name (any case) is refused"

# Bad connections change nothing.
before_list=$(sv connection list)
sed 's/10.77.0.1/10.77.0.999/' "$WORK/a.toml" > "$WORK/bad1.toml"
printf 'version = 1\n[server]\nendpoints = ["10.200.0.1:5000"]\n[routing]\nmode = "split"\ntunnel = ["10.0.0.0/8"]\n[dns]\nmode = "system"\n' > "$WORK/bad2.toml"
{ cat "$WORK/a.toml"; printf '[behavior]\nalways_on = true\n'; } > "$WORK/bad3.toml"
chmod 644 "$WORK"/bad*.toml
refused=0
for b in bad1 bad2 bad3; do
  sv connection add "$WORK/$b.toml" --name "$b" >/dev/null 2>&1 || refused=$((refused + 1))
done
[ "$refused" = 3 ] && [ "$(sv connection list)" = "$before_list" ] && ok=1 || ok=0
result $ok "S08 invalid connections (bad resolver, system DNS, always_on) are refused; nothing changes"

# Connect to A.
out=$(sv connect "Home – Büro A")
grep -q "Kill switch: on" <<<"$out" && grep -q "Connection: c0001" <<<"$out" && ok=1 || ok=0
result $ok "S09 'septuvo connect NAME' selects and connects (kill switch on, connection c0001)"
[ "$(send 5000 tok-a5000)" = sent ] && arrived tok-a5000 && [ "$(send 443 tok-a443)" != sent ] && ok=1 || ok=0
result $ok "S10 under A only A's server port gets out"

rules_a=$(table)
sv connection replace c0001 "$WORK/bad1.toml" >/dev/null 2>&1 && ok=0 || ok=1
[ "$ok" = 1 ] && [ "$(table)" = "$rules_a" ] && ok=1 || ok=0
result $ok "S11 replacing the active connection with a bad one is refused; the rules are unchanged"
sv connection delete c0001 >/dev/null 2>&1 && ok=0 || ok=1
result $ok "S12 the connection in use cannot be deleted while connected"

# T2-SW-01: switch back and forth under load.
rm -f "$WORK/stop" "$WORK/watch.stop"
inC python3 "$HERE/probe.py" flood 10.200.0.1 "$WORK/stop" flood 53 5000 443 > "$WORK/flood.txt" &
FPID=$!
( while [ ! -f "$WORK/watch.stop" ]; do
    if inC nft list table inet septuvo >/dev/null 2>&1; then echo present; else echo MISSING; fi
  done > "$WORK/presence.txt" ) &
WPID=$!
sleep 1
switch_ok=0
for i in $(seq 20); do
  if [ $((i % 2)) = 1 ]; then target=c0002; else target=c0001; fi
  sv connection select "$target" > "$WORK/sw.txt" 2>&1 && grep -q "Service state: blocking" "$WORK/sw.txt" && switch_ok=$((switch_ok + 1))
  sleep 0.1
done
sleep 1
touch "$WORK/stop" "$WORK/watch.stop"
wait "$FPID" 2>/dev/null; FPID=""
wait "$WPID" 2>/dev/null; WPID=""
[ "$switch_ok" = 20 ] && ok=1 || ok=0
result $ok "S13 20 switches while connected all succeeded ($switch_ok/20), state stayed blocking"
leaked=$(grep -c '^flood-53-' "$WORK/u.log")
sent53=$(awk '$1 == 53 {print $2}' "$WORK/flood.txt")
refused53=$(awk '$1 == 53 {print $3}' "$WORK/flood.txt")
[ "$leaked" = 0 ] && [ "${refused53:-0}" -gt 1000 ] && ok=1 || ok=0
result $ok "T2-SW-01a no packet outside both policies got out during the switches (port 53: $leaked arrived, $sent53 sent, $refused53 refused)"
a_arrived=$(grep -c '^flood-5000-' "$WORK/u.log")
b_arrived=$(grep -c '^flood-443-' "$WORK/u.log")
[ "$a_arrived" -gt 0 ] && [ "$b_arrived" -gt 0 ] && ok=1 || ok=0
result $ok "T2-SW-01b each connection's own port got through while it was in use (5000: $a_arrived, 443: $b_arrived)"
missing=$(grep -c MISSING "$WORK/presence.txt")
checks=$(wc -l < "$WORK/presence.txt")
[ "$missing" = 0 ] && [ "$checks" -gt 20 ] && ok=1 || ok=0
result $ok "T2-SW-01c the rules were never missing ($checks checks, $missing missing)"
grep -q "Connection: c0001" <<<"$(sv status)" && [ "$(send 443 tok-end443)" != sent ] && [ "$(send 5000 tok-end5000)" = sent ] && arrived tok-end5000 && ok=1 || ok=0
result $ok "S14 after the last switch (to A) only A's port gets out"

# The selection survives a restart of the service.
{ kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null; DPID=""
start_daemon
out=$(runuser -u "$OTHER" -- "$WORK/bin/septuvo" status --runtime-dir "$RUN")
grep -q "Connection: c0001" <<<"$out" && [ -n "$(table)" ] && ok=1 || ok=0
result $ok "S15 after a service restart the selection is remembered and the rules kept"

# Export, rename, delete.
sv connection export c0002 "$WORK/out/export.toml" >/dev/null && grep -q 'name = "Office B"' "$WORK/out/export.toml" && ok=1 || ok=0
result $ok "S16 export writes a re-importable file with the name"
sv connection export c0002 "$WORK/out/export.toml" >/dev/null 2>&1 && ok=0 || ok=1
result $ok "S17 export never overwrites an existing file"
rules=$(table)
sv connection rename c0001 "Home" >/dev/null && grep -q "c0001  Home$" <<<"$(sv connection list)" && [ "$(table)" = "$rules" ] && ok=1 || ok=0
result $ok "S18 rename changes only the name (rules unchanged)"
sv disconnect >/dev/null
sv connection delete c0002 >/dev/null && ! grep -q c0002 <<<"$(sv connection list)" && [ ! -e "$STORE/c0002.toml" ] && ok=1 || ok=0
result $ok "S19 delete removes a connection and its file"
cp "$WORK/out/export.toml" "$WORK/import.toml"; chmod 644 "$WORK/import.toml"
grep -q "Saved as c0003" <<<"$(sv connection add "$WORK/import.toml")" && grep -q "c0003  Office B" <<<"$(sv connection list)" && ok=1 || ok=0
result $ok "S20 the exported file imports again under its name (IDs are never reused)"
set -e

echo
echo "switch lab: $PASS passed, $FAIL failed (of $N)"
[ "$FAIL" = 0 ]
