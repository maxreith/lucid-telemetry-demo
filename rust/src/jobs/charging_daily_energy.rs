//! Port of scala/.../jobs/ChargingDailyEnergy.scala: energy charged per vehicle per UTC day.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

const MILLIS_PER_DAY: i64 = 86_400_000;

#[derive(Default)]
struct Day<'a> {
    energy_kwh: Option<f64>,
    sessions: HashSet<&'a str>,
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut days: BTreeMap<(&str, String), Day> = BTreeMap::new();
    for e in of_type(dedup(events), "charging_progress") {
        let day = days.entry((e.vin.as_str(), utc_day(e.event_ts))).or_default();
        // Spark's sum skips nulls and stays null when every input is null.
        if let Some(kwh) = e.f64("kwh_delta") {
            *day.energy_kwh.get_or_insert(0.0) += kwh;
        }
        // countDistinct ignores null session ids.
        if let Some(session_id) = e.str("session_id") {
            day.sessions.insert(session_id);
        }
    }
    days.into_iter()
        .map(|((vin, day), d)| {
            json!({
                "vin": vin,
                "day": day,
                "energy_kwh": d.energy_kwh,
                "sessions": d.sessions.len(),
            })
        })
        .collect()
}

/// UTC calendar day of epoch millis as yyyy-MM-dd (Howard Hinnant's civil_from_days).
fn utc_day(ts_millis: i64) -> String {
    let z = ts_millis.div_euclid(MILLIS_PER_DAY) + 719_468;
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
    use serde_json::json;

    fn progress(event_id: &str, event_ts: i64, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": "charging_progress", "event_ts": event_ts,
            "received_ts": event_ts, "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn kwh(event_id: &str, event_ts: i64, session_id: &str, kwh_delta: f64) -> Event {
        progress(event_id, event_ts, json!({"session_id": session_id, "kwh_delta": kwh_delta}))
    }

    #[test]
    fn utc_day_boundaries() {
        assert_eq!(utc_day(0), "1970-01-01");
        assert_eq!(utc_day(MILLIS_PER_DAY - 1), "1970-01-01");
        assert_eq!(utc_day(MILLIS_PER_DAY), "1970-01-02");
        assert_eq!(utc_day(-1), "1969-12-31");
        assert_eq!(utc_day(951_782_400_000), "2000-02-29");
        assert_eq!(utc_day(1_793_491_202_365), "2026-11-01");
    }

    #[test]
    fn resent_events_count_once() {
        let out = run(&[kwh("a", 1, "s1", 1.5), kwh("a", 1, "s1", 1.5), kwh("b", 2, "s1", 2.0)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["energy_kwh"], 3.5);
        assert_eq!(out[0]["sessions"], 1);
    }

    #[test]
    fn session_across_midnight_counts_on_both_days() {
        let out = run(&[
            kwh("a", MILLIS_PER_DAY - 1, "s1", 1.0),
            kwh("b", MILLIS_PER_DAY, "s1", 2.0),
            kwh("c", MILLIS_PER_DAY + 1, "s2", 4.0),
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["day"], "1970-01-01");
        assert_eq!(out[0]["sessions"], 1);
        assert_eq!(out[1]["energy_kwh"], 6.0);
        assert_eq!(out[1]["sessions"], 2);
    }

    #[test]
    fn nulls_are_skipped() {
        let out = run(&[
            progress("a", 1, json!({"session_id": null, "kwh_delta": null})),
            progress("b", 2, json!({})),
        ]);
        assert_eq!(out[0]["energy_kwh"], Value::Null);
        assert_eq!(out[0]["sessions"], 0);
    }

    #[test]
    fn ignores_other_event_types() {
        let mut start = kwh("a", 1, "s1", 1.0);
        start.kind = "charging_start".into();
        assert!(run(&[start]).is_empty());
    }

    #[test]
    fn daily_energy_is_physically_possible() {
        let events = crate::read_events(std::path::Path::new("../data/events.jsonl")).unwrap();
        let progress = of_type(dedup(&events), "charging_progress");
        let total: f64 = progress.iter().filter_map(|e| e.f64("kwh_delta")).sum();
        let out = run(&events);
        let daily_total: f64 = out.iter().filter_map(|r| r["energy_kwh"].as_f64()).sum();
        assert!((total - daily_total).abs() < 1e-6, "energy counted twice or lost");
        for r in &out {
            let kwh = r["energy_kwh"].as_f64().unwrap();
            // Non-negative and below a 350 kW charger running for 24 hours.
            assert!((0.0..=350.0 * 24.0).contains(&kwh), "{r}");
            assert!(r["sessions"].as_u64().unwrap() >= 1, "{r}");
        }
    }
}
