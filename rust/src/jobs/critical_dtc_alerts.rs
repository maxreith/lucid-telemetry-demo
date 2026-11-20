//! Port of scala/.../jobs/CriticalDtcAlerts.scala: diagnostic trouble codes with critical severity.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};

pub fn run(events: &[Event]) -> Vec<Value> {
    of_type(dedup(events), "diagnostic")
        .into_iter()
        .filter(|e| e.str("severity") == Some("critical"))
        .map(|e| {
            json!({
                "vin": e.vin,
                "event_id": e.event_id,
                "event_ts": e.event_ts,
                "code": e.str("code"),
                "ecu": e.str("ecu"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn event(event_id: &str, kind: &str, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": kind, "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn dtc(event_id: &str, severity: &str) -> Event {
        event(
            event_id,
            "diagnostic",
            json!({"code": "P0AA6", "severity": severity, "ecu": "BMS"}),
        )
    }

    #[test]
    fn keeps_only_critical_diagnostics() {
        let out = run(&[
            dtc("a", "critical"),
            dtc("b", "warn"),
            dtc("c", "info"),
            dtc("d", "Critical"),
            event("e", "diagnostic", json!({"code": "P0AA6"})),
            event("f", "tire_pressure", json!({"severity": "critical"})),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0],
            json!({"vin": "VIN1", "event_id": "a", "event_ts": 1, "code": "P0AA6", "ecu": "BMS"})
        );
    }

    #[test]
    fn resent_event_alerts_once() {
        let out = run(&[dtc("a", "critical"), dtc("a", "critical")]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn missing_code_and_ecu_are_null() {
        let out = run(&[event("a", "diagnostic", json!({"severity": "critical"}))]);
        assert_eq!(out[0]["code"], Value::Null);
        assert_eq!(out[0]["ecu"], Value::Null);
    }

    #[test]
    fn no_event_is_alerted_twice_on_recorded_data() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/events.jsonl");
        let Ok(events) = crate::read_events(&path) else {
            return;
        };
        let out = run(&events);
        let ids: HashSet<_> = out.iter().map(|r| r["event_id"].as_str().unwrap()).collect();
        assert_eq!(ids.len(), out.len());
        assert!(out.iter().all(|r| r["code"].is_string() && r["ecu"].is_string()));
    }
}
