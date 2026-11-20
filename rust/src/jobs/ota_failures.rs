//! Port of scala/.../jobs/OtaFailures.scala: one record per failed OTA update, with its error code.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};

/// Spark reads these payload fields with a string schema: a missing or null field is null, any other
/// scalar keeps its JSON text.
fn spark_str(e: &Event, field: &str) -> Value {
    match e.payload.get(field) {
        None | Some(Value::Null) => Value::Null,
        Some(Value::String(s)) => Value::String(s.clone()),
        Some(other) => Value::String(other.to_string()),
    }
}

pub fn run(events: &[Event]) -> Vec<Value> {
    of_type(dedup(events), "ota_status")
        .into_iter()
        .filter(|e| e.str("state") == Some("failed"))
        .map(|e| {
            json!({
                "vin": e.vin,
                "campaign_id": spark_str(e, "campaign_id"),
                "error_code": spark_str(e, "error_code"),
                "event_ts": e.event_ts,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    fn ota(event_id: &str, vin: &str, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "ota_status", "event_ts": 10, "received_ts": 11,
            "fw_version": "2.5.0", "payload": payload
        }))
        .unwrap()
    }

    fn failed(event_id: &str, vin: &str) -> Event {
        ota(
            event_id,
            vin,
            json!({"campaign_id": "C1", "version": "2.6.0", "state": "failed", "error_code": "E_TIMEOUT"}),
        )
    }

    #[test]
    fn keeps_only_failed_states() {
        let mut events: Vec<Event> = ["downloading", "installing", "installed", "FAILED"]
            .iter()
            .enumerate()
            .map(|(i, s)| ota(&i.to_string(), "VIN1", json!({"campaign_id": "C1", "state": s})))
            .collect();
        events.push(failed("f", "VIN1"));
        let out = run(&events);
        assert_eq!(
            out,
            vec![json!({"vin": "VIN1", "campaign_id": "C1", "error_code": "E_TIMEOUT", "event_ts": 10})]
        );
    }

    #[test]
    fn resent_failure_is_reported_once() {
        assert_eq!(run(&[failed("a", "VIN1"), failed("a", "VIN1")]).len(), 1);
    }

    #[test]
    fn ignores_other_event_types_and_missing_state() {
        let mut other = failed("a", "VIN1");
        other.kind = "dtc".into();
        let no_state = ota("b", "VIN1", json!({"campaign_id": "C1", "error_code": "E_X"}));
        assert!(run(&[other, no_state]).is_empty());
    }

    #[test]
    fn missing_fields_are_null() {
        let out = run(&[ota("a", "VIN1", json!({"state": "failed", "error_code": null}))]);
        assert_eq!(out[0]["campaign_id"], Value::Null);
        assert_eq!(out[0]["error_code"], Value::Null);
    }

    #[test]
    fn non_string_error_code_keeps_json_text() {
        let out = run(&[ota("a", "VIN1", json!({"state": "failed", "error_code": 42}))]);
        assert_eq!(out[0]["error_code"], "42");
    }

    #[test]
    fn recorded_failures_are_unique_events() {
        let events = crate::read_events(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/events.jsonl"
        )))
        .unwrap();
        let out = run(&events);
        let failed_ids: HashSet<&str> = dedup(&events)
            .into_iter()
            .filter(|e| e.kind == "ota_status" && e.str("state") == Some("failed"))
            .map(|e| e.event_id.as_str())
            .collect();
        assert_eq!(out.len(), failed_ids.len());
        let keys: HashSet<String> = out
            .iter()
            .map(|r| format!("{}|{}|{}", r["vin"], r["campaign_id"], r["event_ts"]))
            .collect();
        assert_eq!(keys.len(), out.len(), "a failure is counted twice");
        assert!(out.iter().all(|r| r["error_code"].is_string()));
    }
}
