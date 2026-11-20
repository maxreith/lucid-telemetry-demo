//! Port of scala/.../jobs/GeofenceVisits.scala: location pings inside the Newark, CA headquarters geofence per
//! vehicle, with the first and last ping time.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const LAT: (f64, f64) = (37.50, 37.56);
const LON: (f64, f64) = (-122.08, -122.00);

struct Visits {
    pings: i64,
    first_ts: i64,
    last_ts: i64,
}

fn inside(e: &Event) -> bool {
    // Spark's `between` is inclusive on both ends; a null coordinate fails the filter.
    let within = |v: Option<f64>, (lo, hi): (f64, f64)| v.is_some_and(|v| lo <= v && v <= hi);
    within(e.f64("lat"), LAT) && within(e.f64("lon"), LON)
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut by_vin: BTreeMap<&str, Visits> = BTreeMap::new();
    for e in of_type(dedup(events), "location")
        .into_iter()
        .filter(|e| inside(e))
    {
        let v = by_vin.entry(e.vin.as_str()).or_insert(Visits {
            pings: 0,
            first_ts: e.event_ts,
            last_ts: e.event_ts,
        });
        v.pings += 1;
        v.first_ts = v.first_ts.min(e.event_ts);
        v.last_ts = v.last_ts.max(e.event_ts);
    }
    by_vin
        .into_iter()
        .map(|(vin, v)| {
            json!({
                "vin": vin,
                "pings": v.pings,
                "first_ts": v.first_ts,
                "last_ts": v.last_ts,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ping(event_id: &str, vin: &str, ts: i64, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "location", "event_ts": ts, "received_ts": ts,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn at(event_id: &str, vin: &str, ts: i64, lat: f64, lon: f64) -> Event {
        ping(event_id, vin, ts, json!({"lat": lat, "lon": lon}))
    }

    #[test]
    fn counts_pings_once_per_event_id_with_first_and_last_time() {
        let out = run(&[
            at("a", "VIN1", 30, 37.53, -122.03),
            at("a", "VIN1", 30, 37.53, -122.03),
            at("b", "VIN1", 10, 37.52, -122.04),
            at("c", "VIN1", 20, 37.77, -122.42),
        ]);
        assert_eq!(
            out,
            vec![json!({"vin": "VIN1", "pings": 2, "first_ts": 10, "last_ts": 30})]
        );
    }

    #[test]
    fn bounds_are_inclusive() {
        let out = run(&[
            at("a", "VIN1", 1, 37.50, -122.08),
            at("b", "VIN1", 2, 37.56, -122.00),
            at("c", "VIN1", 3, 37.5601, -122.03),
            at("d", "VIN1", 4, 37.53, -121.9999),
        ]);
        assert_eq!(out[0]["pings"], 2);
    }

    #[test]
    fn skips_null_or_missing_coordinates_and_other_types() {
        let mut other = at("c", "VIN1", 3, 37.53, -122.03);
        other.kind = "speed".into();
        let out = run(&[
            ping("a", "VIN1", 1, json!({"lat": null, "lon": -122.03})),
            ping("b", "VIN1", 2, json!({"lat": 37.53})),
            other,
        ]);
        assert!(out.is_empty());
    }

    #[test]
    fn one_record_per_vin_and_pings_never_exceed_location_events() {
        let events = [
            at("a", "VIN1", 1, 37.53, -122.03),
            at("b", "VIN2", 2, 37.53, -122.03),
            at("c", "VIN2", 5, 37.54, -122.02),
        ];
        let out = run(&events);
        assert_eq!(out.len(), 2);
        let total: i64 = out.iter().map(|r| r["pings"].as_i64().unwrap()).sum();
        assert!(total <= events.len() as i64);
        assert!(out
            .iter()
            .all(|r| r["first_ts"].as_i64() <= r["last_ts"].as_i64()));
    }
}
