//! Port of scala/.../jobs/LongTrips.scala: completed trips of 50 km or more.
//! Trips are built like scala/.../jobs/Trips.scala: trip_start and trip_end events grouped by (vin, trip_id).
use crate::{dedup, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const MIN_DISTANCE_KM: f64 = 50.0;

#[derive(Default)]
struct Trip {
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    start_odometer_km: Option<f64>,
    end_odometer_km: Option<f64>,
}

/// Spark's `min`/`max` ignore nulls; the result is null only if every value is null.
fn fold<T: Copy>(acc: Option<T>, v: Option<T>, pick: fn(T, T) -> T) -> Option<T> {
    match (acc, v) {
        (Some(a), Some(b)) => Some(pick(a, b)),
        (a, b) => a.or(b),
    }
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut trips: BTreeMap<(&str, Option<&str>), Trip> = BTreeMap::new();
    for e in dedup(events) {
        let is_start = e.kind == "trip_start";
        if !is_start && e.kind != "trip_end" {
            continue;
        }
        let trip = trips.entry((&e.vin, e.str("trip_id"))).or_default();
        let odometer = e.f64("odometer_km");
        if is_start {
            trip.start_ts = fold(trip.start_ts, Some(e.event_ts), i64::min);
            trip.start_odometer_km = fold(trip.start_odometer_km, odometer, f64::min);
        } else {
            trip.end_ts = fold(trip.end_ts, Some(e.event_ts), i64::max);
            trip.end_odometer_km = fold(trip.end_odometer_km, odometer, f64::max);
        }
    }

    trips
        .into_iter()
        .filter_map(|((vin, trip_id), t)| {
            t.start_ts?;
            t.end_ts?;
            let distance_km = t.end_odometer_km? - t.start_odometer_km?;
            (distance_km >= MIN_DISTANCE_KM).then(|| {
                json!({
                    "vin": vin,
                    "trip_id": trip_id,
                    "start_ts": t.start_ts,
                    "distance_km": distance_km,
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ev(event_id: &str, kind: &str, trip_id: &str, ts: i64, odometer: Option<f64>) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": kind, "event_ts": ts, "received_ts": ts,
            "fw_version": "1.0", "payload": {"trip_id": trip_id, "odometer_km": odometer}
        }))
        .unwrap()
    }

    #[test]
    fn keeps_trips_of_50_km_or_more() {
        let out = run(&[
            ev("a", "trip_start", "t1", 0, Some(1000.0)),
            ev("b", "trip_end", "t1", 3_600_000, Some(1050.0)),
            ev("c", "trip_start", "t2", 0, Some(2000.0)),
            ev("d", "trip_end", "t2", 3_600_000, Some(2049.9)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["trip_id"], "t1");
        assert_eq!(out[0]["start_ts"], 0);
        assert_eq!(out[0]["distance_km"], 50.0);
    }

    #[test]
    fn resent_events_are_counted_once() {
        let out = run(&[
            ev("a", "trip_start", "t1", 0, Some(1000.0)),
            ev("a", "trip_start", "t1", 0, Some(1000.0)),
            ev("b", "trip_end", "t1", 3_600_000, Some(1100.0)),
            ev("b", "trip_end", "t1", 3_600_000, Some(1100.0)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["distance_km"], 100.0);
    }

    #[test]
    fn drops_incomplete_trips_and_null_odometers() {
        let out = run(&[
            ev("a", "trip_start", "open", 0, Some(1000.0)),
            ev("b", "trip_end", "no_start", 3_600_000, Some(1100.0)),
            ev("c", "trip_start", "no_odo", 0, None),
            ev("d", "trip_end", "no_odo", 3_600_000, Some(1100.0)),
        ]);
        assert!(out.is_empty());
    }

    #[test]
    fn aggregates_like_spark_min_max_ignoring_nulls() {
        let out = run(&[
            ev("a", "trip_start", "t1", 10, Some(1000.0)),
            ev("b", "trip_start", "t1", 5, None),
            ev("c", "trip_end", "t1", 3_600_000, Some(1080.0)),
            ev("d", "trip_end", "t1", 3_000_000, Some(1090.0)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["start_ts"], 5);
        assert_eq!(out[0]["distance_km"], 90.0);
    }

    #[test]
    fn distances_are_physically_possible() {
        let events = crate::read_events(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/events.jsonl"
        )));
        let Ok(events) = events else { return };
        let out = run(&events);
        let mut keys = std::collections::HashSet::new();
        for r in &out {
            let d = r["distance_km"].as_f64().unwrap();
            assert!((MIN_DISTANCE_KM..2000.0).contains(&d), "{r}");
            assert!(keys.insert((r["vin"].to_string(), r["trip_id"].to_string())));
        }
    }
}
