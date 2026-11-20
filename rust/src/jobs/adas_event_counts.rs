//! Port of scala/.../jobs/AdasEventCounts.scala: driver-assistance events per vehicle and kind.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut counts: BTreeMap<(&str, Option<&str>), u64> = BTreeMap::new();
    for e in of_type(dedup(events), "adas_event") {
        *counts.entry((e.vin.as_str(), e.str("kind"))).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|((vin, kind), events)| json!({"vin": vin, "kind": kind, "events": events}))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(event_id: &str, vin: &str, kind: &str, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": kind, "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn adas(event_id: &str, vin: &str, kind: &str) -> Event {
        event(event_id, vin, "adas_event", json!({"kind": kind, "speed_kph": 80.0}))
    }

    #[test]
    fn counts_per_vin_and_kind() {
        let out = run(&[
            adas("a", "VIN1", "AEB"),
            adas("b", "VIN1", "AEB"),
            adas("c", "VIN1", "LKA"),
            adas("d", "VIN2", "AEB"),
        ]);
        assert_eq!(
            out,
            vec![
                json!({"vin": "VIN1", "kind": "AEB", "events": 2}),
                json!({"vin": "VIN1", "kind": "LKA", "events": 1}),
                json!({"vin": "VIN2", "kind": "AEB", "events": 1}),
            ]
        );
    }

    #[test]
    fn resent_events_are_counted_once() {
        let out = run(&[adas("a", "VIN1", "BSW"), adas("a", "VIN1", "BSW")]);
        assert_eq!(out, vec![json!({"vin": "VIN1", "kind": "BSW", "events": 1})]);
    }

    #[test]
    fn missing_kind_is_its_own_group() {
        let out = run(&[
            event("a", "VIN1", "adas_event", json!({"speed_kph": 80.0})),
            event("b", "VIN1", "adas_event", json!({"kind": null})),
        ]);
        assert_eq!(out, vec![json!({"vin": "VIN1", "kind": null, "events": 2})]);
    }

    #[test]
    fn ignores_other_event_types() {
        let out = run(&[event("a", "VIN1", "tire_pressure", json!({"kind": "AEB"}))]);
        assert!(out.is_empty());
    }

    #[test]
    fn total_equals_distinct_adas_events() {
        let events = [
            adas("a", "VIN1", "AEB"),
            adas("a", "VIN1", "AEB"),
            adas("b", "VIN2", "FCW"),
            adas("c", "VIN2", "FCW"),
        ];
        let total: u64 = run(&events).iter().map(|r| r["events"].as_u64().unwrap()).sum();
        assert_eq!(total, 3);
    }
}
