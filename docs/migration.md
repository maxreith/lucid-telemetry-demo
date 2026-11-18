# Migrating a job

1. **Port (PR):** Rewrite the Scala job in Rust with the same output. CI replays the recorded events through both
   versions and fails on any difference.
2. **Merge:** After review and green CI.
3. **Staging (CD):** The merged Rust job is built as a container and deployed to the staging Kubernetes cluster.
4. **Shadow run:** In production, the Rust job reads the same Kafka topics as the Scala job with its own consumer
   group and writes to a shadow topic that nothing downstream reads.
5. **Live comparison:** Both outputs are compared for at least two weeks, including restarts and peak load.
6. **Switch-over:** Downstream consumers move to the Rust output, region by region. The Scala job stays on standby.
7. **Retire** the Scala job after 30 clean days.

## Behavior changes

A port never changes behavior, even when the Scala job looks wrong, because downstream systems may depend on
today's numbers. Report suspected bugs separately. A fix goes in its own PR that changes the Scala and Rust
versions together and lists which output records change and why, so the owners of downstream systems can approve
the change.
