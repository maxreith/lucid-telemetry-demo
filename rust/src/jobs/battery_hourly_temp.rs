//! Port of scala/.../jobs/BatteryHourlyTemp.scala: average and maximum cell temperature per vehicle per UTC hour.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const HOUR_MS: i64 = 3_600_000;

#[derive(Default)]
struct Hour {
    sum: f64,
    temps: u64,
    max: Option<f64>,
    samples: u64,
}

/// Start of the UTC hour of an epoch-millis timestamp, like `Events.hourStart`.
fn hour_start(ts: i64) -> i64 {
    ts.div_euclid(HOUR_MS) * HOUR_MS
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut hours: BTreeMap<(&str, i64), Hour> = BTreeMap::new();
    for e in of_type(dedup(events), "battery_status") {
        let h = hours
            .entry((e.vin.as_str(), hour_start(e.event_ts)))
            .or_default();
        // Spark's count(lit(1)) counts every row; avg and max skip null temperatures.
        h.samples += 1;
        if let Some(t) = e.f64("cell_temp_c") {
            h.sum += t;
            h.temps += 1;
            h.max = Some(h.max.map_or(t, |m| m.max(t)));
        }
    }
    hours
        .into_iter()
        .map(|((vin, hour_start_ts), h)| {
            let avg = (h.temps > 0).then(|| h.sum / h.temps as f64);
            json!({
                "vin": vin,
                "hour_start_ts": hour_start_ts,
                "avg_temp_c": avg,
                "max_temp_c": h.max,
                "samples": h.samples,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(event_id: &str, vin: &str, event_ts: i64, temp: Option<f64>) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "battery_status", "event_ts": event_ts,
            "received_ts": event_ts, "fw_version": "1.0", "payload": {"soc": 50.0, "cell_temp_c": temp}
        }))
        .unwrap()
    }

    #[test]
    fn hour_start_floors_to_utc_hour() {
        assert_eq!(hour_start(0), 0);
        assert_eq!(hour_start(HOUR_MS - 1), 0);
        assert_eq!(hour_start(HOUR_MS), HOUR_MS);
        assert_eq!(hour_start(-1), -HOUR_MS);
    }

    #[test]
    fn aggregates_per_vin_and_hour() {
        let out = run(&[
            status("a", "VIN1", 10, Some(20.0)),
            status("b", "VIN1", HOUR_MS - 1, Some(30.0)),
            status("c", "VIN1", HOUR_MS, Some(40.0)),
            status("d", "VIN2", 10, Some(25.0)),
        ]);
        assert_eq!(out.len(), 3);
        assert_eq!(
            out[0],
            json!({"vin": "VIN1", "hour_start_ts": 0, "avg_temp_c": 25.0, "max_temp_c": 30.0, "samples": 2})
        );
        assert_eq!(out[1]["hour_start_ts"], HOUR_MS);
        assert_eq!(out[2]["vin"], "VIN2");
    }

    #[test]
    fn resent_events_are_counted_once() {
        let out = run(&[
            status("a", "VIN1", 10, Some(20.0)),
            status("a", "VIN1", 10, Some(20.0)),
            status("b", "VIN1", 20, Some(40.0)),
        ]);
        assert_eq!(out[0]["samples"], 2);
        assert_eq!(out[0]["avg_temp_c"], 30.0);
    }

    #[test]
    fn null_temperatures_count_as_samples_but_not_in_avg_or_max() {
        let out = run(&[
            status("a", "VIN1", 10, None),
            status("b", "VIN1", 20, Some(30.0)),
            status("c", "VIN2", 10, None),
        ]);
        assert_eq!(
            out[0],
            json!({"vin": "VIN1", "hour_start_ts": 0, "avg_temp_c": 30.0, "max_temp_c": 30.0, "samples": 2})
        );
        assert_eq!(out[1]["avg_temp_c"], Value::Null);
        assert_eq!(out[1]["max_temp_c"], Value::Null);
        assert_eq!(out[1]["samples"], 1);
    }

    #[test]
    fn ignores_other_event_types() {
        let mut other = status("a", "VIN1", 10, Some(99.0));
        other.kind = "charging_session".into();
        assert!(run(&[other]).is_empty());
    }

    #[test]
    fn output_is_physically_plausible() {
        let events = vec![
            status("a", "VIN1", 10, Some(20.0)),
            status("b", "VIN1", 20, Some(35.0)),
            status("c", "VIN1", 30, Some(28.0)),
        ];
        for r in run(&events) {
            let (avg, max) = (r["avg_temp_c"].as_f64().unwrap(), r["max_temp_c"].as_f64().unwrap());
            assert!(avg <= max);
            assert!(r["samples"].as_u64().unwrap() <= events.len() as u64);
        }
    }
}
