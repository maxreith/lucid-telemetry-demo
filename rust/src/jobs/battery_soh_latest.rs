//! Port of scala/.../jobs/BatterySohLatest.scala: latest reported battery state of health per vehicle.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut latest: HashMap<&str, &Event> = HashMap::new();
    for e in of_type(dedup(events), "battery_status") {
        latest
            .entry(e.vin.as_str())
            .and_modify(|best| {
                // Most recent by event time; ties go to the larger event_id.
                if (e.event_ts, e.event_id.as_str()) > (best.event_ts, best.event_id.as_str()) {
                    *best = e;
                }
            })
            .or_insert(e);
    }
    latest
        .values()
        .map(|e| {
            json!({
                "vin": e.vin,
                "soh_pct": e.f64("soh_pct"),
                "event_ts": e.event_ts,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(event_id: &str, vin: &str, kind: &str, event_ts: i64, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": kind, "event_ts": event_ts, "received_ts": event_ts + 5,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn status(event_id: &str, vin: &str, event_ts: i64, soh: f64) -> Event {
        event(event_id, vin, "battery_status", event_ts, json!({"soh_pct": soh}))
    }

    #[test]
    fn keeps_latest_by_event_time_not_arrival_order() {
        let out = run(&[status("b", "VIN1", 200, 91.0), status("a", "VIN1", 100, 95.0)]);
        assert_eq!(out, vec![json!({"vin": "VIN1", "soh_pct": 91.0, "event_ts": 200})]);
    }

    #[test]
    fn ties_go_to_larger_event_id() {
        let out = run(&[status("b", "VIN1", 100, 92.0), status("a", "VIN1", 100, 93.0)]);
        assert_eq!(out[0]["soh_pct"], 92.0);
        let out = run(&[status("a", "VIN1", 100, 93.0), status("b", "VIN1", 100, 92.0)]);
        assert_eq!(out[0]["soh_pct"], 92.0);
    }

    #[test]
    fn one_record_per_vehicle_and_resends_ignored() {
        let out = run(&[
            status("a", "VIN1", 100, 93.0),
            status("a", "VIN1", 100, 93.0),
            status("b", "VIN2", 100, 88.0),
        ]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn ignores_other_event_types() {
        let other = event("z", "VIN1", "charging_session", 999, json!({"soh_pct": 50.0}));
        let out = run(&[status("a", "VIN1", 100, 93.0), other]);
        assert_eq!(out, vec![json!({"vin": "VIN1", "soh_pct": 93.0, "event_ts": 100})]);
    }

    #[test]
    fn missing_soh_is_null_like_spark() {
        let out = run(&[event("a", "VIN1", "battery_status", 100, json!({"soc": 50.0}))]);
        assert_eq!(out[0]["soh_pct"], Value::Null);
    }
}
