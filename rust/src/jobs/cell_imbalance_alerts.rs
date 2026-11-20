//! Port of scala/.../jobs/CellImbalanceAlerts.scala: battery readings whose cell voltage spread exceeds 0.08 V.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};

const THRESHOLD_V: f64 = 0.08;

pub fn run(events: &[Event]) -> Vec<Value> {
    of_type(dedup(events), "battery_status")
        .into_iter()
        .filter_map(|e| {
            // Spark: null if either voltage is missing, and a null spread fails the filter.
            let imbalance_v = e.f64("cell_voltage_max")? - e.f64("cell_voltage_min")?;
            (imbalance_v > THRESHOLD_V).then(|| {
                json!({
                    "vin": e.vin,
                    "event_id": e.event_id,
                    "event_ts": e.event_ts,
                    "imbalance_v": imbalance_v,
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reading(event_id: &str, kind: &str, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": kind, "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn battery(event_id: &str, min: f64, max: f64) -> Event {
        reading(
            event_id,
            "battery_status",
            json!({"cell_voltage_min": min, "cell_voltage_max": max}),
        )
    }

    #[test]
    fn alerts_once_per_imbalanced_reading_even_if_resent() {
        let out = run(&[
            battery("a", 3.60, 3.70),
            battery("a", 3.60, 3.70),
            battery("b", 3.60, 3.65),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["event_id"], "a");
        assert!((out[0]["imbalance_v"].as_f64().unwrap() - 0.10).abs() < 1e-9);
    }

    #[test]
    fn threshold_is_exclusive() {
        assert!(run(&[battery("a", 0.0, THRESHOLD_V)]).is_empty());
    }

    #[test]
    fn missing_voltage_or_other_type_is_skipped() {
        let out = run(&[
            reading("a", "battery_status", json!({"cell_voltage_max": 3.9})),
            reading("b", "battery_status", json!({"cell_voltage_min": 3.5})),
            reading(
                "c",
                "charging",
                json!({"cell_voltage_min": 3.5, "cell_voltage_max": 3.9}),
            ),
        ]);
        assert!(out.is_empty());
    }

    #[test]
    fn imbalance_is_positive_and_physically_plausible() {
        let out = run(&[battery("a", 3.5, 3.7), battery("b", 3.7, 3.5)]);
        assert_eq!(out.len(), 1);
        let v = out[0]["imbalance_v"].as_f64().unwrap();
        assert!(v > THRESHOLD_V && v < 1.0);
    }
}
