#!/usr/bin/env bash
# Septuvo namespace lab (N4-05): saved connections through the desktop app
# against the real lab service: import, select, connect, switch and delete.
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/desktop_connections.sh target/debug/septuvod target/debug/septuvo DESKTOP_TEST_BINARY
#
# DESKTOP_TEST_BINARY is the compiled `lab_connections` test of desktop/
# (cargo test --no-run prints its path). One throw-away namespace runs
# septuvod (lab mode) with an empty private store; the app runs as a
# throw-away member of the control group. The script then checks the
# service itself: the list, the selection and that no rules are left.
set -euo pipefail

usage="usage: desktop_connections.sh PATH/TO/septuvod PATH/TO/septuvo PATH/TO/lab_connections-test"
DAEMON=$(realpath "${1:?$usage}")
CLI=$(realpath "${2:?$usage}")
TEST=$(realpath "${3:?$usage}")
if [ "$(id -u)" != 0 ]; then
  echo "desktop_connections.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft runuser useradd userdel groupadd groupdel realpath; do
  command -v "$tool" >/dev/null || { echo "desktop_connections.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-dn-$ID"
GROUP="sv-ng-$ID" MEMBER="svnm$ID" OTHER="svno$ID" HELPER="svnh$ID"
WORK=$(mktemp -d)
chmod 755 "$WORK"
RUN="$WORK/run"
PASS=0 FAIL=0 N=0
DPID=""

cleanup() {
  [ -n "$DPID" ] && { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null || true
  if ip netns list | grep -qw "$C"; then
    ip netns pids "$C" | xargs -r kill -9 2>/dev/null || true
    ip netns del "$C" 2>/dev/null || true
  fi
  for u in "$MEMBER" "$OTHER" "$HELPER"; do userdel "$u" 2>/dev/null || true; done
  groupdel "$GROUP" 2>/dev/null || true
  groupdel "$HELPER" 2>/dev/null || true
  rm -rf "/etc/netns/$C" "/etc/netns/.septuvo-$C.resolv.conf.backup" "/etc/netns/.septuvo-$C.resolv.conf.tmp" "$WORK"
}
trap cleanup EXIT
trap 'echo "::error title=desktop connections lab setup::desktop_connections.sh line $LINENO failed: $BASH_COMMAND"' ERR

result() {   # $1: 1 = pass, 0 = fail; $2: description
  N=$((N + 1))
  if [ "$1" = 1 ]; then
    PASS=$((PASS + 1)); echo "PASS  $2"
  else
    FAIL=$((FAIL + 1)); echo "FAIL  $2"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=desktop connections lab::$2"; fi
  fi
}

groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
useradd -r -M -U -s /usr/sbin/nologin "$HELPER"
mkdir "$WORK/bin"
install -m 755 "$DAEMON" "$WORK/bin/septuvod"
install -m 755 "$CLI" "$WORK/bin/septuvo"
install -m 755 "$TEST" "$WORK/bin/lab_connections"
ip netns add "$C"
ip -n "$C" link set lo up

cat > "$WORK/septuvo.toml" <<'EOF'
version = 1
[server]
endpoints = ["192.0.2.1:5000"]
[routing]
mode = "full-tunnel"
[dns]
resolvers = ["10.77.0.1"]
EOF
# The two files the app imports (no names inside: the app names them).
sed 's/192.0.2.1:5000/192.0.2.2:5001/' "$WORK/septuvo.toml" > "$WORK/lab-a.toml"
sed 's/192.0.2.1:5000/192.0.2.3:5002/' "$WORK/septuvo.toml" > "$WORK/lab-b.toml"
cat > "$WORK/septuvod.toml" <<EOF
version = 1
[daemon]
mode = "lab"
lab_namespace = "$C"
runtime_dir = "$RUN"
control_group = "$GROUP"
device_config = "$WORK/septuvo.toml"
connections_dir = "$WORK/store/connections"
captive_helper_user = "$HELPER"
EOF
mkdir -m 700 "$WORK/store"
chmod 644 "$WORK"/*.toml

: > "$WORK/daemon.log"
ip netns exec "$C" "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
DPID=$!
ok=0
for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && { ok=1; break; }; sleep 0.1; done
trap - ERR
set +e
result $ok "D00 septuvod starts in the lab namespace"

cli() { runuser -u "$MEMBER" -- "$WORK/bin/septuvo" "$@" --runtime-dir "$RUN"; }

runuser -u "$MEMBER" -- env HOME="$WORK" SEPTUVO_LAB_RUNTIME_DIR="$RUN" \
  SEPTUVO_LAB_FILES="$WORK/lab-a.toml:$WORK/lab-b.toml" \
  "$WORK/bin/lab_connections" --ignored --exact import_select_connect_switch_and_delete_against_the_lab_service \
  --nocapture > "$WORK/app.log" 2>&1
status=$?
grep '^LAB ' "$WORK/app.log"
[ "$status" = 0 ] && ok=1 || { ok=0; tail -40 "$WORK/app.log"; }
result $ok "N01 through the app: import two, select, connect, switch while connected, delete the one in use"
list=$(cli connection list 2>&1)
grep -q "Lab A" <<<"$list" && ! grep -q "Lab B" <<<"$list" && ok=1 || ok=0
result $ok "N02 the service keeps exactly what the app left: Lab A only ($list)"
st=$(cli status 2>&1)
grep -q "Service state: disconnected" <<<"$st" && ! ip netns exec "$C" nft list table inet septuvo >/dev/null 2>&1 && ok=1 || ok=0
result $ok "N03 nothing is left connected and no rules remain"
echo
echo "desktop connections lab: $PASS passed, $FAIL failed (of $N)"
[ "$FAIL" = 0 ]
