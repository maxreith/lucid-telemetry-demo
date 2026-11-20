//! Port of scala/.../jobs/TirePressureDailyMin.scala: lowest pressure per tire per vehicle per UTC day.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const TIRES: [&str; 4] = ["fl", "fr", "rl", "rr"];

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut mins: BTreeMap<(&str, String), [Option<f64>; 4]> = BTreeMap::new();
    for e in of_type(dedup(events), "tire_pressure") {
        let row = mins.entry((e.vin.as_str(), utc_day(e.event_ts))).or_default();
        for (min, tire) in row.iter_mut().zip(TIRES) {
            // Spark's min skips nulls and is null only when every value is null.
            if let Some(kpa) = e.f64(tire) {
                *min = Some(min.map_or(kpa, |m| m.min(kpa)));
            }
        }
    }
    mins.into_iter()
        .map(|((vin, day), [fl, fr, rl, rr])| {
            json!({
                "vin": vin,
                "day": day,
                "min_fl_kpa": fl,
                "min_fr_kpa": fr,
                "min_rl_kpa": rl,
                "min_rr_kpa": rr,
            })
        })
        .collect()
}

/// UTC calendar day of epoch millis as yyyy-MM-dd (Howard Hinnant's civil_from_days).
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

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(event_id: &str, vin: &str, ts: i64, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "tire_pressure", "event_ts": ts, "received_ts": ts,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn tires(fl: f64) -> Value {
        json!({"fl": fl, "fr": 240.0, "rl": 241.0, "rr": 242.0})
    }

    #[test]
    fn utc_day_boundaries() {
        assert_eq!(utc_day(0), "1970-01-01");
        assert_eq!(utc_day(86_399_999), "1970-01-01");
        assert_eq!(utc_day(86_400_000), "1970-01-02");
        assert_eq!(utc_day(-1), "1969-12-31");
        assert_eq!(utc_day(951_782_400_000), "2000-02-29");
        assert_eq!(utc_day(1_793_513_881_000), "2026-11-01");
    }

    #[test]
    fn min_per_vin_and_day() {
        let day = 86_400_000;
        let out = run(&[
            reading("a", "V1", 10, tires(230.0)),
            reading("b", "V1", 20, tires(210.0)),
            reading("c", "V1", day, tires(200.0)),
            reading("d", "V2", 10, tires(220.0)),
        ]);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0]["vin"], "V1");
        assert_eq!(out[0]["day"], "1970-01-01");
        assert_eq!(out[0]["min_fl_kpa"], 210.0);
        assert_eq!(out[0]["min_rr_kpa"], 242.0);
        assert_eq!(out[1]["day"], "1970-01-02");
        assert_eq!(out[1]["min_fl_kpa"], 200.0);
        assert_eq!(out[2]["vin"], "V2");
    }

    #[test]
    fn resent_events_count_once_and_other_types_ignored() {
        let mut other = reading("x", "V1", 10, tires(100.0));
        other.kind = "battery".into();
        let out = run(&[
            reading("a", "V1", 10, tires(230.0)),
            reading("a", "V1", 10, tires(230.0)),
            other,
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["min_fl_kpa"], 230.0);
    }

    #[test]
    fn missing_tire_values_are_skipped_or_null() {
        let out = run(&[
            reading("a", "V1", 10, json!({"fl": null, "fr": 240.0})),
            reading("b", "V1", 20, json!({"fl": 225.0, "fr": 250.0})),
        ]);
        assert_eq!(out[0]["min_fl_kpa"], 225.0);
        assert_eq!(out[0]["min_fr_kpa"], 240.0);
        assert!(out[0]["min_rl_kpa"].is_null());
    }

    #[test]
    fn one_row_per_vin_day_and_minimum_is_a_real_reading() {
        let readings: Vec<Event> = (0..48)
            .map(|i| reading(&format!("e{i}"), "V1", i * 3_600_000, tires(200.0 + i as f64)))
            .collect();
        let out = run(&readings);
        assert_eq!(out.len(), 2);
        for row in &out {
            let kpa = row["min_fl_kpa"].as_f64().unwrap();
            assert!(readings.iter().any(|e| e.f64("fl") == Some(kpa)));
        }
        assert_eq!(out[1]["min_fl_kpa"], 224.0);
    }
}
