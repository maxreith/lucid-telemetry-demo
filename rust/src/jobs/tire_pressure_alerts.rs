//! Port of scala/.../jobs/TirePressureAlerts.scala: one alert per tire reading below 200 kPa.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};

const THRESHOLD_KPA: f64 = 200.0;
const TIRES: [&str; 4] = ["fl", "fr", "rl", "rr"];

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut out = Vec::new();
    for e in of_type(dedup(events), "tire_pressure") {
        for tire in TIRES {
            if let Some(kpa) = e.f64(tire).filter(|&kpa| kpa < THRESHOLD_KPA) {
                out.push(json!({
                    "vin": e.vin,
                    "event_id": e.event_id,
                    "event_ts": e.event_ts,
                    "tire": tire,
                    "pressure_kpa": kpa,
                }));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reading(event_id: &str, fl: f64) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": "tire_pressure", "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": {"fl": fl, "fr": 240.0, "rl": 240.0, "rr": 240.0}
        }))
        .unwrap()
    }

    #[test]
    fn alerts_once_per_low_tire_even_if_resent() {
        let out = run(&[reading("a", 190.0), reading("a", 190.0), reading("b", 230.0)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["tire"], "fl");
    }
}
