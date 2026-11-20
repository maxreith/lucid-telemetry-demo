//! Port of scala/.../jobs/ChargingSessions.scala: one record per charging session with duration, energy
//! delivered, state of charge and peak power.
//!
//! Like the Scala job, this reads the charging events without deduplicating them.
use crate::Event;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Default)]
struct Session {
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    soc_start: Option<f64>,
    soc_end: Option<f64>,
    energy_kwh: Option<f64>,
    peak_power_kw: Option<f64>,
    charger_type: Option<String>,
    pack_capacity_kwh: Option<f64>,
}

fn max_f64(acc: Option<f64>, v: Option<f64>) -> Option<f64> {
    match (acc, v) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut sessions: HashMap<(&str, Option<&str>), Session> = HashMap::new();
    let mut order = Vec::new();
    for e in events {
        if !matches!(
            e.kind.as_str(),
            "charging_start" | "charging_progress" | "charging_end"
        ) {
            continue;
        }
        let key = (e.vin.as_str(), e.str("session_id"));
        let s = sessions.entry(key).or_insert_with(|| {
            order.push(key);
            Session::default()
        });
        match e.kind.as_str() {
            "charging_start" => {
                s.start_ts = Some(s.start_ts.map_or(e.event_ts, |t| t.min(e.event_ts)));
                s.soc_start = s.soc_start.or(e.f64("soc"));
                s.charger_type = s
                    .charger_type
                    .take()
                    .or(e.str("charger_type").map(String::from));
                s.pack_capacity_kwh = s.pack_capacity_kwh.or(e.f64("pack_capacity_kwh"));
            }
            "charging_progress" => {
                if let Some(kwh) = e.f64("kwh_delta") {
                    s.energy_kwh = Some(s.energy_kwh.unwrap_or(0.0) + kwh);
                }
                s.peak_power_kw = max_f64(s.peak_power_kw, e.f64("power_kw"));
            }
            _ => {
                s.end_ts = Some(s.end_ts.map_or(e.event_ts, |t| t.max(e.event_ts)));
                s.soc_end = max_f64(s.soc_end, e.f64("soc"));
            }
        }
    }

    order
        .into_iter()
        .filter_map(|key| {
            let s = &sessions[&key];
            let (start_ts, end_ts) = (s.start_ts?, s.end_ts?);
            Some(json!({
                "vin": key.0,
                "session_id": key.1,
                "start_ts": start_ts,
                "end_ts": end_ts,
                "duration_min": (end_ts - start_ts) as f64 / 60000.0,
                "soc_start": s.soc_start,
                "soc_end": s.soc_end,
                "energy_kwh": s.energy_kwh,
                "peak_power_kw": s.peak_power_kw,
                "charger_type": s.charger_type,
                "pack_capacity_kwh": s.pack_capacity_kwh,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(event_id: &str, kind: &str, ts: i64, payload: Value) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": "VIN1", "type": kind, "event_ts": ts, "received_ts": ts,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn start(id: &str, ts: i64) -> Event {
        event(
            id,
            "charging_start",
            ts,
            json!({"session_id": "s1", "soc": 20.0, "charger_type": "DC", "pack_capacity_kwh": 92.0}),
        )
    }

    fn progress(id: &str, ts: i64, kwh: f64, kw: f64) -> Event {
        event(
            id,
            "charging_progress",
            ts,
            json!({"session_id": "s1", "kwh_delta": kwh, "power_kw": kw}),
        )
    }

    fn end(id: &str, ts: i64, soc: f64) -> Event {
        event(
            id,
            "charging_end",
            ts,
            json!({"session_id": "s1", "soc": soc}),
        )
    }

    #[test]
    fn aggregates_one_session() {
        let out = run(&[
            start("a", 0),
            progress("b", 60_000, 1.5, 50.0),
            progress("c", 120_000, 2.0, 120.0),
            end("d", 180_000, 30.0),
        ]);
        assert_eq!(out.len(), 1);
        let r = &out[0];
        assert_eq!(r["session_id"], "s1");
        assert_eq!(r["start_ts"], 0);
        assert_eq!(r["end_ts"], 180_000);
        assert_eq!(r["duration_min"], 3.0);
        assert_eq!(r["soc_start"], 20.0);
        assert_eq!(r["soc_end"], 30.0);
        assert_eq!(r["energy_kwh"], 3.5);
        assert_eq!(r["peak_power_kw"], 120.0);
        assert_eq!(r["charger_type"], "DC");
        assert_eq!(r["pack_capacity_kwh"], 92.0);
    }

    #[test]
    fn drops_sessions_without_start_or_end() {
        assert!(run(&[start("a", 0), progress("b", 1, 1.0, 7.0)]).is_empty());
        assert!(run(&[progress("b", 1, 1.0, 7.0), end("c", 2, 50.0)]).is_empty());
    }

    #[test]
    fn session_without_progress_has_no_energy() {
        let out = run(&[start("a", 0), end("b", 60_000, 20.0)]);
        assert_eq!(out[0]["energy_kwh"], Value::Null);
        assert_eq!(out[0]["peak_power_kw"], Value::Null);
    }

    #[test]
    fn late_events_use_earliest_start_and_latest_end() {
        let out = run(&[
            end("d", 300_000, 40.0),
            start("a", 60_000),
            end("c", 240_000, 35.0),
            start("b", 0),
        ]);
        assert_eq!(out[0]["start_ts"], 0);
        assert_eq!(out[0]["end_ts"], 300_000);
        assert_eq!(out[0]["soc_end"], 40.0);
    }

    #[test]
    fn sessions_are_keyed_by_vin_and_session_id() {
        let mut other = start("x", 0);
        other.vin = "VIN2".into();
        let mut other_end = end("y", 60_000, 30.0);
        other_end.vin = "VIN2".into();
        let out = run(&[start("a", 0), end("b", 60_000, 30.0), other, other_end]);
        assert_eq!(out.len(), 2);
    }

    /// Matches the Scala job, which does not drop resent events: a resent progress event is counted twice.
    #[test]
    fn resent_progress_is_counted_like_the_scala_job() {
        let p = progress("b", 60_000, 1.5, 50.0);
        let out = run(&[start("a", 0), p.clone(), p, end("d", 120_000, 30.0)]);
        assert_eq!(out[0]["energy_kwh"], 3.0);
    }
}
