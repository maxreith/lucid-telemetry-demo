//! Port of scala/.../jobs/LowSocAlerts.scala: battery readings with state of charge below 15%.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};

const THRESHOLD_PCT: f64 = 15.0;

pub fn run(events: &[Event]) -> Vec<Value> {
    of_type(dedup(events), "battery_status")
        .into_iter()
        .filter_map(|e| {
            let soc = e.f64("soc").filter(|&soc| soc < THRESHOLD_PCT)?;
            Some(json!({
                "vin": e.vin,
                "event_id": e.event_id,
                "event_ts": e.event_ts,
                "soc": soc,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    fn event(event_id: &str, kind: &str, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": kind, "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn battery(event_id: &str, soc: f64) -> Event {
        event(event_id, "battery_status", json!({"soc": soc}))
    }

    #[test]
    fn alerts_once_per_low_reading_even_if_resent() {
        let out = run(&[battery("a", 9.5), battery("a", 9.5), battery("b", 40.0)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["event_id"], "a");
        assert_eq!(out[0]["soc"], 9.5);
    }

    #[test]
    fn threshold_is_exclusive() {
        assert!(run(&[battery("a", 15.0)]).is_empty());
        assert_eq!(run(&[battery("b", 14.99)]).len(), 1);
    }

    #[test]
    fn skips_missing_or_null_soc_and_other_types() {
        let out = run(&[
            event("a", "battery_status", json!({})),
            event("b", "battery_status", json!({"soc": null})),
            event("c", "charging_progress", json!({"soc": 5.0})),
        ]);
        assert!(out.is_empty());
    }

    #[test]
    fn recorded_alerts_are_unique_and_physically_possible() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/events.jsonl");
        let events = crate::read_events(std::path::Path::new(path)).unwrap();
        let out = run(&events);
        assert!(!out.is_empty());
        let ids: HashSet<&str> = out.iter().map(|r| r["event_id"].as_str().unwrap()).collect();
        assert_eq!(ids.len(), out.len());
        for r in &out {
            let soc = r["soc"].as_f64().unwrap();
            assert!((0.0..THRESHOLD_PCT).contains(&soc), "soc out of range: {soc}");
        }
    }
}
