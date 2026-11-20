//! Port of scala/.../jobs/DailyDistance.scala: distance driven and trips per vehicle per UTC day (by trip start).
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

/// UTC calendar day of epoch millis, as yyyy-MM-dd (Events.day).
fn utc_day(ts_ms: i64) -> String {
    let z = ts_ms.div_euclid(86_400_000) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn run(events: &[Event]) -> Vec<Value> {
    // Trips.summaries: pair trip_start and trip_end per (vin, trip_id).
    let mut trips: BTreeMap<(&str, Option<&str>), Trip> = BTreeMap::new();
    for e in dedup(events) {
        let is_start = match e.kind.as_str() {
            "trip_start" => true,
            "trip_end" => false,
            _ => continue,
        };
        let t = trips.entry((&e.vin, e.str("trip_id"))).or_default();
        if is_start {
            t.start_ts = min_opt(t.start_ts, Some(e.event_ts));
            t.start_odometer_km = min_opt(t.start_odometer_km, e.f64("odometer_km"));
        } else {
            t.end_ts = max_opt(t.end_ts, Some(e.event_ts));
            t.end_odometer_km = max_opt(t.end_odometer_km, e.f64("odometer_km"));
        }
    }

    let mut days: BTreeMap<(&str, String), (Option<f64>, u64)> = BTreeMap::new();
    for ((vin, _), t) in trips {
        let (Some(start_ts), Some(_)) = (t.start_ts, t.end_ts) else {
            continue;
        };
        let distance = t.end_odometer_km.zip(t.start_odometer_km).map(|(e, s)| e - s);
        let day = days.entry((vin, utc_day(start_ts))).or_default();
        day.0 = match (day.0, distance) {
            (Some(a), Some(b)) => Some(a + b),
            (a, b) => a.or(b),
        };
        day.1 += 1;
    }

    days.into_iter()
        .map(|((vin, day), (distance_km, trips))| {
            json!({"vin": vin, "day": day, "distance_km": distance_km, "trips": trips})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, vin: &str, kind: &str, ts: i64, trip: &str, odo: Option<f64>) -> Event {
        serde_json::from_value(json!({
            "event_id": id, "vin": vin, "type": kind, "event_ts": ts, "received_ts": ts,
            "fw_version": "1.0", "payload": {"trip_id": trip, "odometer_km": odo}
        }))
        .unwrap()
    }

    const DAY: i64 = 86_400_000;

    #[test]
    fn formats_utc_days() {
        assert_eq!(utc_day(0), "1970-01-01");
        assert_eq!(utc_day(DAY - 1), "1970-01-01");
        assert_eq!(utc_day(951_782_400_000), "2000-02-29");
        assert_eq!(utc_day(1_793_512_980_000), "2026-11-01");
    }

    #[test]
    fn sums_trips_per_day_and_ignores_resent_events() {
        let out = run(&[
            ev("a", "V1", "trip_start", 1000, "t1", Some(100.0)),
            ev("b", "V1", "trip_end", 2000, "t1", Some(110.5)),
            ev("b", "V1", "trip_end", 2000, "t1", Some(110.5)),
            ev("c", "V1", "trip_start", 3000, "t2", Some(110.5)),
            ev("d", "V1", "trip_end", 4000, "t2", Some(120.5)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["day"], "1970-01-01");
        assert_eq!(out[0]["trips"], 2);
        assert!((out[0]["distance_km"].as_f64().unwrap() - 20.5).abs() < 1e-9);
    }

    #[test]
    fn trip_counts_on_start_day_and_needs_both_ends() {
        let out = run(&[
            ev("a", "V1", "trip_start", DAY - 1000, "t1", Some(0.0)),
            ev("b", "V1", "trip_end", DAY + 1000, "t1", Some(5.0)),
            ev("c", "V1", "trip_start", DAY + 2000, "open", Some(5.0)),
            ev("d", "V2", "trip_end", DAY + 3000, "orphan", Some(9.0)),
        ]);
        assert_eq!(out, vec![json!({"vin": "V1", "day": "1970-01-01", "distance_km": 5.0, "trips": 1})]);
    }

    #[test]
    fn missing_odometer_gives_null_distance_but_counts_trip() {
        let out = run(&[
            ev("a", "V1", "trip_start", 0, "t1", None),
            ev("b", "V1", "trip_end", 10, "t1", Some(5.0)),
        ]);
        assert_eq!(out[0]["distance_km"], Value::Null);
        assert_eq!(out[0]["trips"], 1);
    }

    #[test]
    fn recorded_data_is_physically_plausible() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/events.jsonl");
        let events = crate::read_events(&path).unwrap();
        for r in run(&events) {
            let km = r["distance_km"].as_f64().unwrap();
            assert!((0.0..=2400.0).contains(&km), "implausible daily distance: {r}");
            assert!(r["trips"].as_u64().unwrap() >= 1);
        }
    }
}
