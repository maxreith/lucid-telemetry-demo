#!/usr/bin/env bash
# Replays the recorded events through the Scala and Rust versions of each job and compares the outputs.
# Usage: tools/replay.sh [job ...]   (no jobs = every job ported to Rust)
set -euo pipefail
cd "$(dirname "$0")/.."

EVENTS=data/events.jsonl
OUT=out

(cd rust && cargo build --release --quiet)
if [ $# -eq 0 ]; then
  mapfile -t JOBS < <(rust/target/release/telemetry-job --list)
else
  JOBS=("$@")
fi

echo "== Scala/Spark (${#JOBS[@]} jobs)"
(cd scala && sbt -batch -error "runMain lucid.telemetry.Replay ../$EVENTS ../$OUT/scala ${JOBS[*]}")
echo "== Rust"
rust/target/release/telemetry-job "$EVENTS" "$OUT/rust" "${JOBS[@]}"
echo "== Compare"
python3 tools/compare.py "$OUT/scala" "$OUT/rust" "${JOBS[@]}"
