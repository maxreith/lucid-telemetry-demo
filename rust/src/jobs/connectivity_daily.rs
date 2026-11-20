//! Port of scala/.../jobs/ConnectivityDaily.scala: average signal strength and share of 5G samples per vehicle
//! per UTC day.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Default)]
struct Day {
    signal_sum: f64,
    signal_count: u64,
    samples_5g: u64,
    samples: u64,
}

/// UTC calendar day of epoch millis, as yyyy-MM-dd (Spark `date_format(timestamp_millis(ts), "yyyy-MM-dd")`).
fn utc_day(ts_millis: i64) -> String {
    let z = ts_millis.div_euclid(86_400_000) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut days: BTreeMap<(&str, String), Day> = BTreeMap::new();
    for e in of_type(dedup(events), "connectivity") {
        let day = days.entry((&e.vin, utc_day(e.event_ts))).or_default();
        // Spark `avg` skips nulls; `when(network === "5G", 1.0).otherwise(0.0)` counts a null network as 0.
        if let Some(dbm) = e.f64("signal_dbm") {
            day.signal_sum += dbm;
            day.signal_count += 1;
        }
        day.samples_5g += u64::from(e.str("network") == Some("5G"));
        day.samples += 1;
    }
    days.into_iter()
        .map(|((vin, day), d)| {
            let avg_signal_dbm =
                (d.signal_count > 0).then(|| d.signal_sum / d.signal_count as f64);
            json!({
                "vin": vin,
                "day": day,
                "avg_signal_dbm": avg_signal_dbm,
                "share_5g": d.samples_5g as f64 / d.samples as f64,
                "samples": d.samples,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample(event_id: &str, vin: &str, event_ts: i64, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "connectivity", "event_ts": event_ts,
            "received_ts": event_ts, "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    #[test]
    fn utc_day_boundaries() {
        assert_eq!(utc_day(0), "1970-01-01");
        assert_eq!(utc_day(86_399_999), "1970-01-01");
        assert_eq!(utc_day(86_400_000), "1970-01-02");
        assert_eq!(utc_day(-1), "1969-12-31");
        assert_eq!(utc_day(1_709_164_800_000), "2024-02-29");
        assert_eq!(utc_day(1_793_513_882_000), "2026-11-01");
    }

    #[test]
    fn averages_per_vin_and_day_counting_resends_once() {
        let a = sample("a", "VIN1", 1_000, json!({"signal_dbm": -80.0, "network": "5G"}));
        let out = run(&[
            a.clone(),
            a,
            sample("b", "VIN1", 2_000, json!({"signal_dbm": -100.0, "network": "LTE"})),
            sample("c", "VIN1", 86_400_000, json!({"signal_dbm": -90.0, "network": "LTE"})),
            sample("d", "VIN2", 1_000, json!({"signal_dbm": -70.0, "network": "5G"})),
        ]);
        assert_eq!(out.len(), 3);
        assert_eq!(
            out[0],
            json!({"vin": "VIN1", "day": "1970-01-01", "avg_signal_dbm": -90.0, "share_5g": 0.5, "samples": 2})
        );
        assert_eq!(out[1]["day"], "1970-01-02");
        assert_eq!(out[1]["share_5g"], 0.0);
        assert_eq!(out[2]["vin"], "VIN2");
    }

    #[test]
    fn null_signal_skipped_in_avg_and_null_network_counts_as_not_5g() {
        let out = run(&[
            sample("a", "VIN1", 1_000, json!({"network": "5G"})),
            sample("b", "VIN1", 2_000, json!({"signal_dbm": -90.0})),
        ]);
        assert_eq!(out[0]["avg_signal_dbm"], -90.0);
        assert_eq!(out[0]["share_5g"], 0.5);
        assert_eq!(out[0]["samples"], 2);

        let out = run(&[sample("a", "VIN1", 1_000, json!({}))]);
        assert_eq!(out[0]["avg_signal_dbm"], Value::Null);
        assert_eq!(out[0]["share_5g"], 0.0);
    }

    #[test]
    fn ignores_other_event_types() {
        let mut e = sample("a", "VIN1", 1_000, json!({"signal_dbm": -80.0}));
        e.kind = "gps".into();
        assert!(run(&[e]).is_empty());
    }

    #[test]
    fn outputs_are_physically_possible_on_recorded_data() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/events.jsonl");
        let events = crate::read_events(&path).unwrap();
        let out = run(&events);
        let unique = dedup(&events).into_iter().filter(|e| e.kind == "connectivity").count() as u64;
        let total: u64 = out.iter().map(|r| r["samples"].as_u64().unwrap()).sum();
        assert_eq!(total, unique, "every sample counted exactly once");
        for r in &out {
            let share = r["share_5g"].as_f64().unwrap();
            assert!((0.0..=1.0).contains(&share));
            assert!(r["samples"].as_u64().unwrap() >= 1);
            let dbm = r["avg_signal_dbm"].as_f64().unwrap();
            assert!((-150.0..=-30.0).contains(&dbm), "implausible signal {dbm}");
        }
    }
}
