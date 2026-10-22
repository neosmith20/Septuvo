#!/usr/bin/env bash
# Septuvo namespace lab (N4-01): the desktop app's window X against the real
# lab service.
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/desktop_close.sh target/debug/septuvod target/debug/septuvo DESKTOP_TEST_BINARY
#
# DESKTOP_TEST_BINARY is the compiled `lab_close` test of desktop/ (cargo test
# --no-run prints its path). One throw-away namespace runs septuvod (lab
# mode). For each case the service is connected (kill switch blocking), then
# the test presses the app's X:
#   member  in the control group: the app disconnects, then exits;
#   other   not in it: disconnect is refused, the window stays with the
#           error, and a second X exits without changing the service.
set -euo pipefail

usage="usage: desktop_close.sh PATH/TO/septuvod PATH/TO/septuvo PATH/TO/lab_close-test"
DAEMON=$(realpath "${1:?$usage}")
CLI=$(realpath "${2:?$usage}")
TEST=$(realpath "${3:?$usage}")
if [ "$(id -u)" != 0 ]; then
  echo "desktop_close.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft runuser useradd userdel groupadd groupdel realpath; do
  command -v "$tool" >/dev/null || { echo "desktop_close.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-dc-$ID"
GROUP="sv-dg-$ID" MEMBER="svdm$ID" OTHER="svdo$ID" HELPER="svdh$ID"
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
trap 'echo "::error title=desktop close lab setup::desktop_close.sh line $LINENO failed: $BASH_COMMAND"' ERR

result() {   # $1: 1 = pass, 0 = fail; $2: description
  N=$((N + 1))
  if [ "$1" = 1 ]; then
    PASS=$((PASS + 1)); echo "PASS  $2"
  else
    FAIL=$((FAIL + 1)); echo "FAIL  $2"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=desktop close lab::$2"; fi
  fi
}

groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
useradd -r -M -U -s /usr/sbin/nologin "$HELPER"
mkdir "$WORK/bin"
install -m 755 "$DAEMON" "$WORK/bin/septuvod"
install -m 755 "$CLI" "$WORK/bin/septuvo"
install -m 755 "$TEST" "$WORK/bin/lab_close"
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
cat > "$WORK/septuvod.toml" <<EOF
version = 1
[daemon]
mode = "lab"
lab_namespace = "$C"
runtime_dir = "$RUN"
control_group = "$GROUP"
device_config = "$WORK/septuvo.toml"
connections_dir = "$WORK/connections"
captive_helper_user = "$HELPER"
EOF
chmod 644 "$WORK"/*.toml

: > "$WORK/daemon.log"
ip netns exec "$C" "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
DPID=$!
ok=0
for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && { ok=1; break; }; sleep 0.1; done
trap - ERR
set +e
result $ok "D00 septuvod starts in the lab namespace"

cli() { runuser -u "$MEMBER" -- "$WORK/bin/septuvo" "$@" --runtime-dir "$RUN" > /dev/null; }

for case in member other; do
  cli connect
  user=$MEMBER
  [ "$case" = other ] && user=$OTHER
  # The app runs as an ordinary user, as it does on a desktop.
  runuser -u "$user" -- env HOME="$WORK" SEPTUVO_LAB_RUNTIME_DIR="$RUN" SEPTUVO_LAB_CASE="$case" \
    "$WORK/bin/lab_close" --ignored --exact the_x_against_the_lab_service --nocapture > "$WORK/$case.log" 2>&1
  status=$?
  grep '^LAB ' "$WORK/$case.log"
  [ "$status" = 0 ] && ok=1 || { ok=0; cat "$WORK/$case.log"; }
  if [ "$case" = member ]; then
    result $ok "D01 member: the X disconnects (service reports disconnected) and then exits"
  else
    result $ok "D02 not in the control group: disconnect refused, window stays with the error; second X exits, service unchanged"
  fi
  cli disconnect
done

echo
echo "desktop close lab: $PASS passed, $FAIL failed (of $N)"
[ "$FAIL" = 0 ]
