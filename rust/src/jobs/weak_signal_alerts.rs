//! Port of scala/.../jobs/WeakSignalAlerts.scala: connectivity samples with signal below -110 dBm.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};

const THRESHOLD_DBM: f64 = -110.0;

pub fn run(events: &[Event]) -> Vec<Value> {
    of_type(dedup(events), "connectivity")
        .into_iter()
        .filter_map(|e| {
            let dbm = e.f64("signal_dbm").filter(|&dbm| dbm < THRESHOLD_DBM)?;
            Some(json!({
                "vin": e.vin,
                "event_id": e.event_id,
                "event_ts": e.event_ts,
                "signal_dbm": dbm,
                "network": e.str("network"),
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    fn sample(event_id: &str, kind: &str, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": kind, "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn signal(event_id: &str, dbm: f64) -> Event {
        sample(event_id, "connectivity", json!({"signal_dbm": dbm, "network": "LTE"}))
    }

    #[test]
    fn alerts_once_per_weak_sample_even_if_resent() {
        let out = run(&[signal("a", -115.0), signal("a", -115.0), signal("b", -90.0)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["event_id"], "a");
        assert_eq!(out[0]["signal_dbm"], -115.0);
        assert_eq!(out[0]["network"], "LTE");
    }

    #[test]
    fn threshold_is_strict() {
        assert!(run(&[signal("a", -110.0)]).is_empty());
        assert_eq!(run(&[signal("a", -110.1)]).len(), 1);
    }

    #[test]
    fn skips_missing_signal_and_other_types() {
        let out = run(&[
            sample("a", "connectivity", json!({"network": "5G"})),
            sample("b", "connectivity", json!({"signal_dbm": null, "network": "5G"})),
            sample("c", "battery_status", json!({"signal_dbm": -120.0})),
        ]);
        assert!(out.is_empty());
    }

    #[test]
    fn missing_network_is_null() {
        let out = run(&[sample("a", "connectivity", json!({"signal_dbm": -120.0}))]);
        assert_eq!(out.len(), 1);
        assert!(out[0]["network"].is_null());
    }

    #[test]
    fn recorded_alerts_are_unique_and_plausible() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/events.jsonl");
        let out = run(&crate::read_events(&path).unwrap());
        assert!(!out.is_empty());
        let ids: HashSet<_> = out.iter().map(|r| r["event_id"].as_str().unwrap()).collect();
        assert_eq!(ids.len(), out.len(), "an event is alerted twice");
        for r in &out {
            let dbm = r["signal_dbm"].as_f64().unwrap();
            assert!((-150.0..-110.0).contains(&dbm), "implausible signal {dbm}");
        }
    }
}
