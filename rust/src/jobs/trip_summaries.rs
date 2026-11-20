//! Port of scala/.../jobs/TripSummaries.scala (via Trips.summaries): distance, duration and average speed per
//! completed trip, pairing trip_start and trip_end events by (vin, trip_id).
use crate::{dedup, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Default)]
struct Trip {
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    start_odometer_km: Option<f64>,
    end_odometer_km: Option<f64>,
}

fn min_opt<T: PartialOrd>(acc: Option<T>, v: Option<T>) -> Option<T> {
    match (acc, v) {
        (Some(a), Some(b)) => Some(if b < a { b } else { a }),
        (a, b) => a.or(b),
    }
}

fn max_opt<T: PartialOrd>(acc: Option<T>, v: Option<T>) -> Option<T> {
    match (acc, v) {
        (Some(a), Some(b)) => Some(if b > a { b } else { a }),
        (a, b) => a.or(b),
    }
}

/// Spark (non-ANSI) division: null when either side is null or the divisor is zero.
fn div(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) if b != 0.0 => Some(a / b),
        _ => None,
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
            trip.start_ts = min_opt(trip.start_ts, Some(e.event_ts));
            trip.start_odometer_km = min_opt(trip.start_odometer_km, odometer);
        } else {
            trip.end_ts = max_opt(trip.end_ts, Some(e.event_ts));
            trip.end_odometer_km = max_opt(trip.end_odometer_km, odometer);
        }
    }

    trips
        .into_iter()
        .filter_map(|((vin, trip_id), t)| {
            let (start_ts, end_ts) = (t.start_ts?, t.end_ts?);
            let distance_km = t.end_odometer_km.zip(t.start_odometer_km).map(|(e, s)| e - s);
            let duration_min = (end_ts - start_ts) as f64 / 60000.0;
            let avg_speed_kph = div(distance_km, Some(duration_min / 60.0));
            Some(json!({
                "vin": vin,
                "trip_id": trip_id,
                "start_ts": start_ts,
                "end_ts": end_ts,
                "distance_km": distance_km,
                "duration_min": duration_min,
                "avg_speed_kph": avg_speed_kph,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, vin: &str, kind: &str, ts: i64, trip_id: Option<&str>, odo: Option<f64>) -> Event {
        serde_json::from_value(json!({
            "event_id": id, "vin": vin, "type": kind, "event_ts": ts, "received_ts": ts,
            "fw_version": "1.0", "payload": {"trip_id": trip_id, "odometer_km": odo}
        }))
        .unwrap()
    }

    #[test]
    fn summarizes_completed_trip() {
        let out = run(&[
            ev("a", "V1", "trip_start", 0, Some("t1"), Some(100.0)),
            ev("b", "V1", "trip_end", 1_800_000, Some("t1"), Some(130.0)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["distance_km"], 30.0);
        assert_eq!(out[0]["duration_min"], 30.0);
        assert_eq!(out[0]["avg_speed_kph"], 60.0);
        assert_eq!(out[0]["start_ts"], 0);
        assert_eq!(out[0]["end_ts"], 1_800_000);
    }

    #[test]
    fn resent_events_count_once_and_incomplete_trips_are_dropped() {
        let out = run(&[
            ev("a", "V1", "trip_start", 0, Some("t1"), Some(100.0)),
            ev("a", "V1", "trip_start", 0, Some("t1"), Some(100.0)),
            ev("b", "V1", "trip_end", 60_000, Some("t1"), Some(101.0)),
            ev("b", "V1", "trip_end", 60_000, Some("t1"), Some(101.0)),
            ev("c", "V1", "trip_start", 0, Some("t2"), Some(200.0)),
            ev("d", "V2", "trip_end", 0, Some("t3"), Some(5.0)),
            ev("e", "V2", "speed", 0, Some("t3"), Some(1.0)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["trip_id"], "t1");
        assert_eq!(out[0]["distance_km"], 1.0);
    }

    #[test]
    fn groups_by_vin_and_trip_id_using_earliest_start_and_latest_end() {
        let out = run(&[
            ev("a", "V1", "trip_start", 60_000, Some("t1"), Some(10.0)),
            ev("b", "V1", "trip_start", 0, Some("t1"), Some(12.0)),
            ev("c", "V1", "trip_end", 120_000, Some("t1"), Some(15.0)),
            ev("d", "V1", "trip_end", 180_000, Some("t1"), Some(14.0)),
            ev("e", "V2", "trip_start", 0, Some("t1"), Some(0.0)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["start_ts"], 0);
        assert_eq!(out[0]["end_ts"], 180_000);
        assert_eq!(out[0]["distance_km"], 5.0);
        assert_eq!(out[0]["duration_min"], 3.0);
    }

    #[test]
    fn nulls_follow_spark_semantics() {
        let out = run(&[
            ev("a", "V1", "trip_start", 0, None, None),
            ev("b", "V1", "trip_end", 0, None, Some(5.0)),
        ]);
        assert_eq!(out.len(), 1);
        assert!(out[0]["trip_id"].is_null());
        assert!(out[0]["distance_km"].is_null());
        assert_eq!(out[0]["duration_min"], 0.0);
        assert!(out[0]["avg_speed_kph"].is_null());

        let out = run(&[
            ev("a", "V1", "trip_start", 0, Some("t"), Some(1.0)),
            ev("b", "V1", "trip_end", 0, Some("t"), Some(1.0)),
        ]);
        assert!(out[0]["avg_speed_kph"].is_null(), "division by zero is null in Spark");
    }

    #[test]
    fn recorded_trips_are_physically_possible() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/events.jsonl");
        let events = crate::read_events(&path).unwrap();
        let out = run(&events);
        assert!(!out.is_empty());
        let mut keys = std::collections::HashSet::new();
        for r in &out {
            assert!(keys.insert((r["vin"].to_string(), r["trip_id"].to_string())), "trip counted twice");
            assert!(r["end_ts"].as_i64() > r["start_ts"].as_i64());
            let distance = r["distance_km"].as_f64().unwrap();
            assert!(distance >= 0.0, "negative distance: {r}");
            let speed = r["avg_speed_kph"].as_f64().unwrap();
            assert!((0.0..=250.0).contains(&speed), "implausible speed: {r}");
        }
    }
}
