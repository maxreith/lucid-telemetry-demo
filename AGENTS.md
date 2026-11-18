# Agent notes

- To port a job, follow `.agents/skills/port-scala-job/SKILL.md`.
- The Scala jobs are the source of truth for behavior. Never change them in a port.
- One job per PR. A port PR adds `rust/src/jobs/<job_name>.rs` and nothing shared, so ports can run in parallel
  without merge conflicts. If a shared helper in `rust/src/event.rs` is missing, put it in your job file.
- Before opening a PR, `tools/replay.sh <job_name>` must print `MATCH`, and `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass (run them in `rust/`).
- Do not edit `data/events.jsonl` or `tools/compare.py` to make a comparison pass.
- Outputs contain VINs and locations. Never copy recorded data into issues, PR descriptions or logs beyond a few
  example records.
