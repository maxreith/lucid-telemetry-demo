---
name: port-scala-job
description: Port one legacy Scala/Spark telemetry job to Rust with proven output parity.
---

# Port a Scala/Spark job to Rust

Input: one job name, e.g. `battery_soh_latest`.

1. Read `scala/src/main/scala/lucid/telemetry/jobs/<JobName>.scala` and every helper it calls (`Events.scala`,
   `Trips.scala`). Note exactly which events it reads, how it treats duplicates, late events and nulls, and what
   it outputs. Spark semantics matter: `first`, `max` over nulls, `avg` over integers, rounding, day boundaries
   in UTC.
2. Write `rust/src/jobs/<job_name>.rs` exposing `pub fn run(events: &[Event]) -> Vec<serde_json::Value>`.
   Use `rust/src/jobs/tire_pressure_alerts.rs` as the reference. Output field names and types must match the
   Scala output. Do not edit shared files.
3. Add unit tests in the same file for the edge cases you found in step 1.
4. Run `tools/replay.sh <job_name>` until it prints `MATCH`. When it shows a diff, find the cause in the Scala
   semantics; never change the Scala job, the data or the comparison to make it pass.
5. Check the output makes sense for the business: write tests for rules that must hold, e.g. counts and totals
   are physically possible and nothing is counted twice. If a rule fails on both Scala and Rust, the bug is in
   the legacy job: keep the port faithful, and report it.
6. Open a PR titled `Port <job_name> to Rust`. In the description: what the job does, the replay result, and any
   suspected legacy bug with evidence (how many records are affected, one example).
7. If you found a legacy bug, open a second PR, based on the port branch, that fixes it in both Scala and Rust.
   List how the output changes (records changed, totals before and after) so downstream owners can approve it.
