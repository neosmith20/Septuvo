#!/usr/bin/env bash
# Runs cargo-fuzz targets for a fixed time each and reports failures as
# GitHub annotations (H-018). Used by .github/workflows/fuzz.yml; also works
# locally from the repository root:
#   fuzz/ci-run.sh SECONDS TARGET...      (needs a nightly toolchain and cargo-fuzz)
# Environment: FUZZ_TOOLCHAIN (default "nightly"), FUZZ_CARGO (default "cargo").
# Exit status: 0 if every target ran for its time without a failure.
set -euo pipefail
seconds=${1:?usage: ci-run.sh SECONDS TARGET...}
shift
case "$seconds" in ''|*[!0-9]*) echo "SECONDS must be a number" >&2; exit 2 ;; esac
cd "$(dirname "$0")/.."   # the repository root; cargo-fuzz gets --fuzz-dir fuzz
toolchain=${FUZZ_TOOLCHAIN:-nightly}
cargo=${FUZZ_CARGO:-cargo}
status=0
summary=${GITHUB_STEP_SUMMARY:-/dev/null}
for target in "$@"; do
  case "$target" in *[!a-z_]*|'') echo "bad target name: $target" >&2; exit 2 ;; esac
  mkdir -p "fuzz/corpus/$target"
  log="fuzz/$target.log"
  rc=0
  # -max_len matches the standard-library runner; -timeout catches hangs.
  "$cargo" "+$toolchain" fuzz run --fuzz-dir fuzz "$target" "fuzz/corpus/$target" -- \
    -max_total_time="$seconds" -max_len=4096 -timeout=10 -rss_limit_mb=2048 -print_final_stats=1 \
    > "$log" 2>&1 || rc=$?
  runs=$(grep -oE 'stat::number_of_executed_units: *[0-9]+' "$log" | grep -oE '[0-9]+$' | tail -1 || true)
  cov=$(grep -oE 'cov: [0-9]+' "$log" | tail -1 | grep -oE '[0-9]+' || true)
  if [ "$rc" = 0 ] && [ -n "$runs" ]; then
    line="$target: ${runs} executions in ${seconds}s, coverage ${cov:-?} edges, no failures"
    echo "$line"
    echo "- $line" >> "$summary"
  else
    status=1
    echo "::error title=libFuzzer $target::exit $rc; see the job log"
    grep -E "panicked at|assertion|left:|right:|ERROR: libFuzzer|SUMMARY:" "$log" | head -8 | while IFS= read -r l; do
      echo "::error title=libFuzzer $target::$l"
    done
    for crash in fuzz/artifacts/"$target"/*; do
      [ -f "$crash" ] || continue
      echo "::error title=libFuzzer $target input::$crash hex $(head -c 300 "$crash" | od -An -tx1 | tr -d ' \n')"
      echo "::error title=libFuzzer $target replay::cargo +nightly fuzz run --fuzz-dir fuzz $target $crash"
    done
    tail -n 20 "$log"
    echo "- $target: FAILED (exit $rc)" >> "$summary"
  fi
done
exit "$status"
