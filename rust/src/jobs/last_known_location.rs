//! Port of scala/.../jobs/LastKnownLocation.scala: most recent location per vehicle, by event time.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn run(events: &[Event]) -> Vec<Value> {
    latest_by_vin(of_type(dedup(events), "location"))
        .into_iter()
        .map(|e| {
            json!({
                "vin": e.vin,
                "lat": e.f64("lat"),
                "lon": e.f64("lon"),
                "event_ts": e.event_ts,
            })
        })
        .collect()
}

/// Events.latestBy(_, "vin"): most recent event per VIN by event time; ties go to the larger event_id.
fn latest_by_vin(events: Vec<&Event>) -> Vec<&Event> {
    let mut latest: HashMap<&str, &Event> = HashMap::new();
    for e in events {
        latest
            .entry(e.vin.as_str())
            .and_modify(|cur| {
                if (e.event_ts, e.event_id.as_str()) > (cur.event_ts, cur.event_id.as_str()) {
                    *cur = e;
                }
            })
            .or_insert(e);
    }
    latest.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(event_id: &str, vin: &str, kind: &str, event_ts: i64, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": kind, "event_ts": event_ts, "received_ts": event_ts,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn loc(event_id: &str, vin: &str, event_ts: i64, lat: f64) -> Event {
        event(event_id, vin, "location", event_ts, json!({"lat": lat, "lon": -122.0}))
    }

    #[test]
    fn keeps_latest_by_event_time_not_arrival_order() {
        let out = run(&[loc("b", "V1", 200, 2.0), loc("a", "V1", 100, 1.0)]);
        assert_eq!(out, vec![json!({"vin": "V1", "lat": 2.0, "lon": -122.0, "event_ts": 200})]);
    }

    #[test]
    fn ties_go_to_larger_event_id() {
        let out = run(&[loc("b", "V1", 100, 2.0), loc("a", "V1", 100, 1.0)]);
        assert_eq!(out[0]["lat"], 2.0);
        let out = run(&[loc("a", "V1", 100, 1.0), loc("b", "V1", 100, 2.0)]);
        assert_eq!(out[0]["lat"], 2.0);
    }

    #[test]
    fn one_record_per_vehicle_and_only_location_events() {
        let out = run(&[
            loc("a", "V1", 100, 1.0),
            loc("a", "V1", 100, 1.0),
            loc("b", "V2", 50, 3.0),
            event("c", "V1", "tire_pressure", 900, json!({"fl": 240.0})),
            event("d", "V3", "charging", 900, json!({"soc": 50.0})),
        ]);
        assert_eq!(out.len(), 2);
        let v1 = out.iter().find(|r| r["vin"] == "V1").unwrap();
        assert_eq!(v1["event_ts"], 100);
    }

    #[test]
    fn missing_coordinates_are_null() {
        let out = run(&[event("a", "V1", "location", 100, json!({"speed_kph": 10.0}))]);
        assert_eq!(out[0]["lat"], Value::Null);
        assert_eq!(out[0]["lon"], Value::Null);
    }
}
