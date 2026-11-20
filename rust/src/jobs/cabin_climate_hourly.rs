//! Port of scala/.../jobs/CabinClimateHourly.scala: average cabin and outside temperature per vehicle per UTC hour.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const HOUR_MS: i64 = 3_600_000;

/// Spark `avg`: nulls are skipped, and a group with only nulls averages to null.
#[derive(Default)]
struct Avg {
    sum: f64,
    count: u64,
}

impl Avg {
    fn add(&mut self, value: Option<f64>) {
        if let Some(v) = value {
            self.sum += v;
            self.count += 1;
        }
    }

    fn value(&self) -> Option<f64> {
        (self.count > 0).then(|| self.sum / self.count as f64)
    }
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut groups: BTreeMap<(&str, i64), (Avg, Avg)> = BTreeMap::new();
    for e in of_type(dedup(events), "climate") {
        let hour_start_ts = e.event_ts.div_euclid(HOUR_MS) * HOUR_MS;
        let (cabin, outside) = groups.entry((e.vin.as_str(), hour_start_ts)).or_default();
        cabin.add(e.f64("cabin_temp_c"));
        outside.add(e.f64("outside_temp_c"));
    }
    groups
        .into_iter()
        .map(|((vin, hour_start_ts), (cabin, outside))| {
            json!({
                "vin": vin,
                "hour_start_ts": hour_start_ts,
                "avg_cabin_temp_c": cabin.value(),
                "avg_outside_temp_c": outside.value(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const HOUR: i64 = 1_793_512_800_000;

    fn reading(event_id: &str, vin: &str, event_ts: i64, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "climate", "event_ts": event_ts,
            "received_ts": event_ts + 1000, "fw_version": "2.5.2", "payload": payload
        }))
        .unwrap()
    }

    fn temps(cabin: f64, outside: f64) -> Value {
        json!({"cabin_temp_c": cabin, "outside_temp_c": outside})
    }

    #[test]
    fn averages_per_vin_and_utc_hour() {
        let out = run(&[
            reading("a", "VIN1", HOUR, temps(20.0, 10.0)),
            reading("b", "VIN1", HOUR + HOUR_MS - 1, temps(22.0, 14.0)),
            reading("c", "VIN1", HOUR + HOUR_MS, temps(30.0, 30.0)),
            reading("d", "VIN2", HOUR + 5, temps(18.0, 5.0)),
        ]);
        assert_eq!(out.len(), 3);
        assert_eq!(
            out[0],
            json!({"vin": "VIN1", "hour_start_ts": HOUR, "avg_cabin_temp_c": 21.0, "avg_outside_temp_c": 12.0})
        );
        assert_eq!(out[1]["hour_start_ts"], HOUR + HOUR_MS);
        assert_eq!(out[2]["vin"], "VIN2");
    }

    #[test]
    fn resent_events_count_once() {
        let out = run(&[
            reading("a", "VIN1", HOUR, temps(20.0, 10.0)),
            reading("a", "VIN1", HOUR, temps(20.0, 10.0)),
            reading("b", "VIN1", HOUR + 1, temps(23.0, 13.0)),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["avg_cabin_temp_c"], 21.5);
        assert_eq!(out[0]["avg_outside_temp_c"], 11.5);
    }

    #[test]
    fn nulls_are_skipped_and_all_null_averages_to_null() {
        let out = run(&[
            reading("a", "VIN1", HOUR, json!({"cabin_temp_c": 20.0})),
            reading("b", "VIN1", HOUR + 1, json!({"cabin_temp_c": 24.0, "outside_temp_c": null})),
        ]);
        assert_eq!(out[0]["avg_cabin_temp_c"], 22.0);
        assert_eq!(out[0]["avg_outside_temp_c"], Value::Null);
    }

    #[test]
    fn ignores_other_event_types() {
        let mut other = reading("a", "VIN1", HOUR, temps(20.0, 10.0));
        other.kind = "battery_status".into();
        assert!(run(&[other]).is_empty());
    }

    #[test]
    fn averages_stay_within_the_readings_range() {
        let events: Vec<Event> = (0..10)
            .map(|i| reading(&i.to_string(), "VIN1", HOUR + i, temps(19.0 + i as f64 / 2.0, 12.0 + i as f64)))
            .collect();
        let out = run(&events);
        let cabin = out[0]["avg_cabin_temp_c"].as_f64().unwrap();
        assert!((19.0..=23.5).contains(&cabin));
    }
}
