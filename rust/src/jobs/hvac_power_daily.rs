//! Port of scala/.../jobs/HvacPowerDaily.scala: average and peak HVAC power per vehicle per UTC day.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const MS_PER_DAY: i64 = 86_400_000;

#[derive(Default)]
struct Acc {
    sum: f64,
    count: u64,
    max: Option<f64>,
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut groups: BTreeMap<(&str, String), Acc> = BTreeMap::new();
    for e in of_type(dedup(events), "climate") {
        let acc = groups.entry((e.vin.as_str(), utc_day(e.event_ts))).or_default();
        if let Some(kw) = e.f64("hvac_power_kw") {
            acc.sum += kw;
            acc.count += 1;
            acc.max = Some(acc.max.map_or(kw, |m| m.max(kw)));
        }
    }
    groups
        .into_iter()
        .map(|((vin, day), acc)| {
            let avg = (acc.count > 0).then(|| acc.sum / acc.count as f64);
            json!({
                "vin": vin,
                "day": day,
                "avg_hvac_power_kw": avg,
                "max_hvac_power_kw": acc.max,
            })
        })
        .collect()
}

/// UTC calendar day of epoch millis as yyyy-MM-dd (Howard Hinnant's civil_from_days).
fn utc_day(ts_ms: i64) -> String {
    let z = ts_ms.div_euclid(MS_PER_DAY) + 719_468;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn climate(event_id: &str, vin: &str, event_ts: i64, kw: Option<f64>) -> Event {
        let payload = match kw {
            Some(kw) => json!({"cabin_temp_c": 21.0, "hvac_power_kw": kw}),
            None => json!({"cabin_temp_c": 21.0}),
        };
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "climate", "event_ts": event_ts,
            "received_ts": event_ts, "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    #[test]
    fn utc_day_boundaries() {
        assert_eq!(utc_day(0), "1970-01-01");
        assert_eq!(utc_day(MS_PER_DAY - 1), "1970-01-01");
        assert_eq!(utc_day(MS_PER_DAY), "1970-01-02");
        assert_eq!(utc_day(-1), "1969-12-31");
        assert_eq!(utc_day(951_782_400_000), "2000-02-29");
        assert_eq!(utc_day(1_793_513_583_000), "2026-11-01");
    }

    #[test]
    fn resent_events_count_once() {
        let out = run(&[
            climate("a", "VIN1", 1_000, Some(1.0)),
            climate("a", "VIN1", 1_000, Some(1.0)),
            climate("b", "VIN1", 2_000, Some(2.0)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["avg_hvac_power_kw"], 1.5);
        assert_eq!(out[0]["max_hvac_power_kw"], 2.0);
    }

    #[test]
    fn groups_by_vin_and_utc_day() {
        let out = run(&[
            climate("a", "VIN1", MS_PER_DAY - 1, Some(1.0)),
            climate("b", "VIN1", MS_PER_DAY, Some(2.0)),
            climate("c", "VIN2", MS_PER_DAY, Some(3.0)),
        ]);
        assert_eq!(out.len(), 3);
        assert!(out.iter().any(|r| r["vin"] == "VIN1" && r["day"] == "1970-01-01"));
    }

    #[test]
    fn missing_power_is_ignored_and_all_missing_gives_nulls() {
        let out = run(&[
            climate("a", "VIN1", 0, None),
            climate("b", "VIN1", 0, Some(3.0)),
            climate("c", "VIN2", 0, None),
        ]);
        let vin1 = out.iter().find(|r| r["vin"] == "VIN1").unwrap();
        assert_eq!(vin1["avg_hvac_power_kw"], 3.0);
        let vin2 = out.iter().find(|r| r["vin"] == "VIN2").unwrap();
        assert!(vin2["avg_hvac_power_kw"].is_null() && vin2["max_hvac_power_kw"].is_null());
    }

    #[test]
    fn ignores_other_event_types() {
        let mut e = climate("a", "VIN1", 0, Some(1.0));
        e.kind = "charging".into();
        assert!(run(&[e]).is_empty());
    }

    #[test]
    fn recorded_data_is_physically_plausible() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/events.jsonl");
        let Ok(events) = crate::read_events(&path) else { return };
        for r in run(&events) {
            let avg = r["avg_hvac_power_kw"].as_f64().unwrap();
            let max = r["max_hvac_power_kw"].as_f64().unwrap();
            assert!((0.0..=avg + 1e-9).contains(&avg) && avg <= max + 1e-9, "{r}");
            assert!(max < 20.0, "{r}");
        }
    }
}
