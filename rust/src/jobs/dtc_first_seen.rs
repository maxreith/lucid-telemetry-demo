//! Port of scala/.../jobs/DtcFirstSeen.scala: first and last time each diagnostic trouble code was seen per vehicle.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut seen: BTreeMap<(&str, Option<&str>), (i64, i64)> = BTreeMap::new();
    for e in of_type(dedup(events), "diagnostic") {
        seen.entry((e.vin.as_str(), e.str("code")))
            .and_modify(|(first, last)| {
                *first = (*first).min(e.event_ts);
                *last = (*last).max(e.event_ts);
            })
            .or_insert((e.event_ts, e.event_ts));
    }
    seen.into_iter()
        .map(|((vin, code), (first, last))| {
            json!({"vin": vin, "code": code, "first_seen_ts": first, "last_seen_ts": last})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(event_id: &str, vin: &str, kind: &str, code: Option<&str>, ts: i64) -> Event {
        let payload = match code {
            Some(c) => json!({"code": c, "severity": "critical", "ecu": "BMS"}),
            None => json!({"severity": "critical", "ecu": "BMS"}),
        };
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": kind, "event_ts": ts, "received_ts": ts,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    #[test]
    fn first_and_last_per_vin_and_code_regardless_of_order() {
        let out = run(&[
            event("a", "VIN1", "diagnostic", Some("P0AA6"), 30),
            event("b", "VIN1", "diagnostic", Some("P0AA6"), 10),
            event("c", "VIN1", "diagnostic", Some("P0AA6"), 20),
            event("d", "VIN1", "diagnostic", Some("U0100"), 5),
            event("e", "VIN2", "diagnostic", Some("P0AA6"), 7),
        ]);
        assert_eq!(out.len(), 3);
        assert_eq!(
            out[0],
            json!({"vin": "VIN1", "code": "P0AA6", "first_seen_ts": 10, "last_seen_ts": 30})
        );
        assert_eq!(
            out[1],
            json!({"vin": "VIN1", "code": "U0100", "first_seen_ts": 5, "last_seen_ts": 5})
        );
        assert_eq!(
            out[2],
            json!({"vin": "VIN2", "code": "P0AA6", "first_seen_ts": 7, "last_seen_ts": 7})
        );
    }

    #[test]
    fn ignores_resent_copies_and_other_event_types() {
        let out = run(&[
            event("a", "VIN1", "diagnostic", Some("P0AA6"), 10),
            event("a", "VIN1", "diagnostic", Some("P0AA6"), 10),
            event("b", "VIN1", "battery_status", Some("P0AA6"), 1),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["first_seen_ts"], 10);
    }

    #[test]
    fn missing_code_is_its_own_null_group() {
        let out = run(&[
            event("a", "VIN1", "diagnostic", None, 10),
            event("b", "VIN1", "diagnostic", None, 20),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["code"], Value::Null);
        assert_eq!(out[0]["last_seen_ts"], 20);
    }

    #[test]
    fn first_seen_never_after_last_seen_and_one_record_per_key() {
        let events: Vec<Event> = (0..50)
            .map(|i| {
                let code = ["P0AA6", "B1342", "U0293"][i % 3];
                let vin = ["VIN1", "VIN2"][i % 2];
                event(&format!("e{i}"), vin, "diagnostic", Some(code), (i as i64 * 7919) % 1000)
            })
            .collect();
        let out = run(&events);
        assert_eq!(out.len(), 6);
        for r in &out {
            assert!(r["first_seen_ts"].as_i64() <= r["last_seen_ts"].as_i64());
        }
    }
}
