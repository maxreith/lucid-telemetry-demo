# Lucid vehicle telemetry: Scala/Spark → Rust migration

Thirty telemetry jobs, written in Scala on Spark, turn raw vehicle events from Kafka into the records that the
service, warranty and OTA systems read. They are being moved to Rust services on Kubernetes, one job at a time,
without changing what downstream systems receive.

| Path | What it is |
|---|---|
| `scala/` | The 30 legacy Spark jobs (`src/main/scala/lucid/telemetry/jobs/`). `Replay` runs them in batch over recorded events. |
| `rust/` | The Rust target. Each file in `src/jobs/` is one ported job and is registered automatically. |
| `data/events.jsonl` | Recorded Kafka traffic: 50 vehicles over 3 days, including resent duplicates and late events. Fake data. |
| `tools/replay.sh` | Runs the Scala and Rust versions of each ported job on the recorded events and compares the outputs. |
| `tools/compare.py` | Record-by-record comparison: missing, extra or changed records fail the check. |
| `docs/migration.md` | How a job moves from Scala to production Rust. |

## Status

1 of 30 jobs ported: `tire_pressure_alerts`. List ported jobs with `cargo run -q --manifest-path rust/Cargo.toml -- --list`.

## Run

Requirements: Java 17, sbt, Rust stable, Python 3.

```bash
tools/replay.sh                       # every ported job
tools/replay.sh tire_pressure_alerts  # one job
cd rust && cargo test
```

CI runs `cargo fmt`, `clippy`, `cargo test` and the replay parity check on every pull request.
