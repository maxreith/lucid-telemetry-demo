//! Port of scala/.../jobs/SpeedingEvents.scala: location pings above 120 km/h.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};

const LIMIT_KPH: f64 = 120.0;

pub fn run(events: &[Event]) -> Vec<Value> {
    of_type(dedup(events), "location")
        .into_iter()
        .filter_map(|e| {
            let kph = e.f64("speed_kph").filter(|&kph| kph > LIMIT_KPH)?;
            Some(json!({
                "vin": e.vin,
                "event_id": e.event_id,
                "event_ts": e.event_ts,
                "speed_kph": kph,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(event_id: &str, kind: &str, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": kind, "event_ts": 7, "received_ts": 9,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn ping(event_id: &str, kph: f64) -> Event {
        event(event_id, "location", json!({"lat": 1.0, "lon": 2.0, "speed_kph": kph, "heading": 90}))
    }

    #[test]
    fn outputs_pings_strictly_above_limit() {
        let out = run(&[ping("a", 120.0), ping("b", 120.1), ping("c", 80.0)]);
        assert_eq!(
            out,
            vec![json!({"vin": "VIN1", "event_id": "b", "event_ts": 7, "speed_kph": 120.1})]
        );
    }

    #[test]
    fn resent_ping_counts_once() {
        let out = run(&[ping("a", 150.0), ping("a", 150.0)]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn ignores_missing_speed_and_other_types() {
        let out = run(&[
            event("a", "location", json!({"lat": 1.0, "lon": 2.0})),
            event("b", "location", json!({"speed_kph": null})),
            event("c", "hvac", json!({"speed_kph": 200.0})),
        ]);
        assert!(out.is_empty());
    }

    #[test]
    fn integer_speed_is_read_as_double() {
        let out = run(&[event("a", "location", json!({"speed_kph": 130}))]);
        assert_eq!(out[0]["speed_kph"], json!(130.0));
    }
}
