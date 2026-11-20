//! Port of scala/.../jobs/DcFastChargeCounts.scala: number of DC fast-charging sessions per vehicle.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
    for e in of_type(dedup(events), "charging_start") {
        if e.str("charger_type") == Some("DC") {
            *counts.entry(e.vin.as_str()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(vin, n)| json!({"vin": vin, "dc_sessions": n}))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(event_id: &str, vin: &str, kind: &str, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": kind, "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn start(event_id: &str, vin: &str, session: &str, charger: Value) -> Event {
        event(
            event_id,
            vin,
            "charging_start",
            json!({"session_id": session, "charger_type": charger, "soc": 20.0}),
        )
    }

    #[test]
    fn counts_dc_starts_per_vin_once_even_if_resent() {
        let out = run(&[
            start("a", "VIN1", "s1", json!("DC")),
            start("a", "VIN1", "s1", json!("DC")),
            start("b", "VIN1", "s2", json!("DC")),
            start("c", "VIN2", "s3", json!("DC")),
        ]);
        assert_eq!(
            out,
            vec![
                json!({"vin": "VIN1", "dc_sessions": 2}),
                json!({"vin": "VIN2", "dc_sessions": 1}),
            ]
        );
    }

    #[test]
    fn ignores_ac_missing_charger_type_and_other_event_types() {
        let out = run(&[
            start("a", "VIN1", "s1", json!("AC")),
            start("b", "VIN1", "s2", Value::Null),
            start("c", "VIN1", "s3", json!("dc")),
            event("d", "VIN1", "charging_end", json!({"session_id": "s4", "charger_type": "DC"})),
            event("e", "VIN1", "charging_start", json!({"session_id": "s5"})),
        ]);
        assert!(out.is_empty());
    }

}
