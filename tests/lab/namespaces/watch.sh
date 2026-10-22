#!/usr/bin/env bash
# Septuvo namespace lab (N3-04): the live status view, `septuvo watch`.
#
# Usage (root, on a disposable lab machine or CI runner only):
#   sudo tests/lab/namespaces/watch.sh target/debug/septuvod target/debug/septuvo
#
# One throw-away namespace runs septuvod (lab mode). `septuvo watch` runs as an
# ordinary user (not in the control group, so it can only read status) and
# refreshes every second while the service is connected, opens a
# captive-portal window, disconnects, is killed and is restarted. The output
# (plain text: not a terminal) must follow every change, show a lost service
# at once and never claim "Connected".
set -euo pipefail

DAEMON=$(realpath "${1:?usage: watch.sh PATH/TO/septuvod PATH/TO/septuvo}")
CLI=$(realpath "${2:?usage: watch.sh PATH/TO/septuvod PATH/TO/septuvo}")
if [ "$(id -u)" != 0 ]; then
  echo "watch.sh: needs root (namespaces, test users); use a disposable lab VM or CI runner" >&2
  exit 2
fi
for tool in ip nft python3 runuser useradd userdel groupadd groupdel realpath; do
  command -v "$tool" >/dev/null || { echo "watch.sh: missing $tool" >&2; exit 2; }
done

ID=$$
C="sv-wc-$ID"
GROUP="sv-wg-$ID" MEMBER="svwm$ID" OTHER="svwo$ID" HELPER="svwh$ID"
WORK=$(mktemp -d)
chmod 755 "$WORK"
RUN="$WORK/run"
PASS=0 FAIL=0 N=0
DPID="" WPID=""

cleanup() {
  [ -n "$WPID" ] && kill "$WPID" 2>/dev/null || true
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
trap 'echo "::error title=watch lab setup::watch.sh line $LINENO failed: $BASH_COMMAND"' ERR

result() {   # $1: 1 = pass, 0 = fail; $2: description
  N=$((N + 1))
  if [ "$1" = 1 ]; then
    PASS=$((PASS + 1)); echo "PASS  $2"
  else
    FAIL=$((FAIL + 1)); echo "FAIL  $2"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=watch lab::$2"; fi
  fi
}

groupadd "$GROUP"
useradd -M -N -s /usr/sbin/nologin -G "$GROUP" "$MEMBER"
useradd -M -N -s /usr/sbin/nologin "$OTHER"
useradd -r -M -U -s /usr/sbin/nologin "$HELPER"
mkdir "$WORK/bin"
install -m 755 "$DAEMON" "$WORK/bin/septuvod"
install -m 755 "$CLI" "$WORK/bin/septuvo"
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

start_daemon() {
  : > "$WORK/daemon.log"
  ip netns exec "$C" "$WORK/bin/septuvod" --settings "$WORK/septuvod.toml" >> "$WORK/daemon.log" 2>&1 &
  DPID=$!
  for _ in $(seq 50); do [ -S "$RUN/status.sock" ] && grep -q ready "$WORK/daemon.log" && return 0; sleep 0.1; done
  return 1
}
cli() { runuser -u "$MEMBER" -- "$WORK/bin/septuvo" "$@" --runtime-dir "$RUN" > /dev/null; }
now() { python3 -c 'import time; print(f"{time.monotonic():.2f}")'; }

trap - ERR
set +e
start_daemon && ok=1 || ok=0
result $ok "W00 septuvod starts in the lab namespace"

# The watcher: an ordinary user, plain output to a file, one refresh a second.
runuser -u "$OTHER" -- "$WORK/bin/septuvo" watch --interval 1 --runtime-dir "$RUN" > "$WORK/watch.log" 2>&1 &
WPID=$!
T0=$(now)
mark() { echo "$1 $(python3 -c "print(round($(now) - $T0, 2))")" >> "$WORK/marks.txt"; }
sleep 4
mark connect;    cli connect;                     sleep 4
mark captive;    cli captive-portal --minutes 1;  sleep 4
mark disconnect; cli disconnect;                  sleep 4
mark kill;       { kill -9 "$DPID"; wait "$DPID"; } 2>/dev/null; DPID=""; rm -rf "$RUN"; sleep 4
mark restart;    start_daemon;                    sleep 4
mark end
kill "$WPID" 2>/dev/null; wait "$WPID" 2>/dev/null; WPID=""

# Split the output into frames and check each phase.
python3 - "$WORK/watch.log" "$WORK/marks.txt" > "$WORK/checks.txt" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
marks = dict((k, float(v)) for k, v in (l.split() for l in open(sys.argv[2])))
frames = []
for chunk in re.split(r"(?m)^(?=Septuvo watch: refresh )", text):
    m = re.match(r"Septuvo watch: refresh (\d+), (\d+)s", chunk)
    if m:
        frames.append((int(m.group(2)), chunk))
def phase(start, end, pred, label):
    # Frames safely inside the phase. A frame shown as "Ns" was drawn at some
    # time in [N, N+1): it must start after the change (plus a refresh) and
    # end before the next one.
    inside = [f for t, f in frames if marks[start] + 1.2 <= t and t + 1 <= marks[end]]
    ok = bool(inside) and all(pred(f) for f in inside)
    print(f"{int(ok)} {label} ({len(inside)} frames)")
before = [f for t, f in frames if t + 1 <= marks["connect"]]
print(f"{int(bool(before) and all('Kill switch: off' in f for f in before))} W01 before connect: kill switch off ({len(before)} frames)")
phase("connect", "captive", lambda f: "Kill switch: on" in f and "Captive portal: closed" in f, "W02 after connect: kill switch on, captive portal closed")
phase("captive", "disconnect", lambda f: "Captive portal: OPEN" in f and "left)" in f and "WARNING:" in f, "W03 captive-portal window: OPEN with countdown and warning")
phase("disconnect", "kill", lambda f: "Kill switch: off" in f and "Captive portal: OPEN" not in f, "W04 after disconnect: kill switch off, window closed")
phase("kill", "restart", lambda f: "SERVICE LOST" in f and "Kill switch" not in f and "Captive portal" not in f, "W05 service killed: SERVICE LOST at once, no last known state")
phase("restart", "end", lambda f: "SERVICE LOST" not in f and "Kill switch:" in f, "W06 service restarted: status shown again")
print(f"{int(len(frames) >= 20)} W07 the view kept refreshing ({len(frames)} frames)")
print(f"{int(not any('Status: Connected' in f for _, f in frames))} W08 no frame ever claims Connected")
print(f"{int(chr(27) not in text)} W09 plain output when not a terminal (no escape sequences)")
counts = re.findall(r"Captive portal: OPEN \((\d+):(\d\d) left\)", text)
secs = [int(m) * 60 + int(s) for m, s in counts]
print(f"{int(len(secs) >= 2 and secs == sorted(secs, reverse=True) and secs[0] <= 60)} W10 the countdown counts down ({secs[:4]}...)")
PY
while read -r ok desc; do result "$ok" "$desc"; done < "$WORK/checks.txt"
grep -q "permission\|not_permitted" "$WORK/watch.log" && ok=0 || ok=1
result $ok "W11 an ordinary user can watch (status socket only, read-only)"
[ "$FAIL" = 0 ] || { echo "---- watch output (first 60 lines) ----"; head -60 "$WORK/watch.log"; cat "$WORK/marks.txt"; }

echo
echo "watch lab: $PASS passed, $FAIL failed (of $N)"
[ "$FAIL" = 0 ]
