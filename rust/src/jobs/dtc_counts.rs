//! Port of scala/.../jobs/DtcCounts.scala: occurrences of each diagnostic trouble code per vehicle.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut counts: BTreeMap<(&str, Option<&str>), i64> = BTreeMap::new();
    for e in of_type(dedup(events), "diagnostic") {
        *counts.entry((e.vin.as_str(), e.str("code"))).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|((vin, code), occurrences)| json!({"vin": vin, "code": code, "occurrences": occurrences}))
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

    fn dtc(event_id: &str, vin: &str, code: &str) -> Event {
        event(event_id, vin, "diagnostic", json!({"code": code, "severity": "critical"}))
    }

    fn occurrences(out: &[Value], vin: &str, code: Value) -> Option<i64> {
        out.iter()
            .find(|r| r["vin"] == vin && r["code"] == code)
            .map(|r| r["occurrences"].as_i64().unwrap())
    }

    #[test]
    fn counts_per_vin_and_code() {
        let out = run(&[
            dtc("a", "VIN1", "P0AA6"),
            dtc("b", "VIN1", "P0AA6"),
            dtc("c", "VIN1", "P1E00"),
            dtc("d", "VIN2", "P0AA6"),
        ]);
        assert_eq!(out.len(), 3);
        assert_eq!(occurrences(&out, "VIN1", json!("P0AA6")), Some(2));
        assert_eq!(occurrences(&out, "VIN1", json!("P1E00")), Some(1));
        assert_eq!(occurrences(&out, "VIN2", json!("P0AA6")), Some(1));
    }

    #[test]
    fn resent_events_are_counted_once() {
        let out = run(&[dtc("a", "VIN1", "P0AA6"), dtc("a", "VIN1", "P0AA6")]);
        assert_eq!(occurrences(&out, "VIN1", json!("P0AA6")), Some(1));
    }

    #[test]
    fn ignores_other_event_types() {
        let out = run(&[event("a", "VIN1", "tire_pressure", json!({"code": "P0AA6"}))]);
        assert!(out.is_empty());
    }

    #[test]
    fn missing_code_is_its_own_group() {
        let out = run(&[event("a", "VIN1", "diagnostic", json!({})), dtc("b", "VIN1", "P0AA6")]);
        assert_eq!(occurrences(&out, "VIN1", Value::Null), Some(1));
        assert_eq!(occurrences(&out, "VIN1", json!("P0AA6")), Some(1));
    }

    #[test]
    fn total_occurrences_equal_distinct_diagnostic_events() {
        let events = [
            dtc("a", "VIN1", "P0AA6"),
            dtc("a", "VIN1", "P0AA6"),
            dtc("b", "VIN2", "P1E00"),
            event("c", "VIN1", "tire_pressure", json!({})),
        ];
        let total: i64 = run(&events).iter().map(|r| r["occurrences"].as_i64().unwrap()).sum();
        assert_eq!(total, 2);
        assert!(run(&events).iter().all(|r| r["occurrences"].as_i64().unwrap() >= 1));
    }
}
