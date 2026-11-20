//! Port of scala/.../jobs/BatteryTempAlerts.scala: battery readings with a cell temperature above 45 C.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};

const THRESHOLD_C: f64 = 45.0;

pub fn run(events: &[Event]) -> Vec<Value> {
    of_type(dedup(events), "battery_status")
        .into_iter()
        .filter_map(|e| {
            let temp = e.f64("cell_temp_c").filter(|&t| t > THRESHOLD_C)?;
            Some(json!({
                "vin": e.vin,
                "event_id": e.event_id,
                "event_ts": e.event_ts,
                "cell_temp_c": temp,
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

    fn reading(event_id: &str, temp: f64) -> Event {
        event(event_id, "battery_status", json!({"cell_temp_c": temp}))
    }

    #[test]
    fn alerts_only_strictly_above_threshold() {
        let out = run(&[reading("a", 45.0), reading("b", 45.01), reading("c", 30.0)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["event_id"], "b");
        assert_eq!(out[0]["cell_temp_c"], 45.01);
    }

    #[test]
    fn resent_reading_alerts_once() {
        let out = run(&[reading("a", 50.0), reading("a", 50.0)]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn missing_or_null_temp_and_other_types_are_ignored() {
        let out = run(&[
            event("a", "battery_status", json!({})),
            event("b", "battery_status", json!({"cell_temp_c": null})),
            event("c", "hvac_status", json!({"cell_temp_c": 60.0})),
        ]);
        assert!(out.is_empty());
    }

    #[test]
    fn integer_temp_is_read_as_double() {
        let out = run(&[event("a", "battery_status", json!({"cell_temp_c": 50}))]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["cell_temp_c"].as_f64(), Some(50.0));
    }

    #[test]
    fn recorded_alerts_are_unique_and_physically_plausible() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/events.jsonl");
        let events = crate::read_events(&path).unwrap();
        let out = run(&events);
        let ids: HashSet<_> = out.iter().map(|r| r["event_id"].as_str().unwrap()).collect();
        assert_eq!(ids.len(), out.len());
        assert!(out.iter().all(|r| {
            let t = r["cell_temp_c"].as_f64().unwrap();
            t > THRESHOLD_C && t < 100.0
        }));
    }
}
