//! Port of scala/.../jobs/ChargingPeakPower.scala: peak and average charging power per session.
use crate::{dedup, of_type, Event};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

#[derive(Default)]
struct Session {
    peak: Option<f64>,
    sum: f64,
    count: u64,
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut sessions: BTreeMap<(&str, Option<&str>), Session> = BTreeMap::new();
    for e in of_type(dedup(events), "charging_progress") {
        let s = sessions
            .entry((e.vin.as_str(), e.str("session_id")))
            .or_default();
        if let Some(kw) = e.f64("power_kw") {
            s.peak = Some(s.peak.map_or(kw, |p| p.max(kw)));
            s.sum += kw;
            s.count += 1;
        }
    }
    sessions
        .into_iter()
        .map(|((vin, session_id), s)| {
            // Spark omits null columns from toJSON output.
            let mut rec = Map::new();
            rec.insert("vin".into(), json!(vin));
            if let Some(id) = session_id {
                rec.insert("session_id".into(), json!(id));
            }
            if let Some(peak) = s.peak {
                rec.insert("peak_power_kw".into(), json!(peak));
                rec.insert("avg_power_kw".into(), json!(s.sum / s.count as f64));
            }
            Value::Object(rec)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn progress(event_id: &str, vin: &str, session_id: Option<&str>, power_kw: Option<f64>) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "charging_progress", "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": {"session_id": session_id, "kwh_delta": 0.5, "power_kw": power_kw}
        }))
        .unwrap()
    }

    fn find<'a>(out: &'a [Value], vin: &str, session_id: &str) -> &'a Value {
        out.iter()
            .find(|r| r["vin"] == vin && r["session_id"] == session_id)
            .unwrap()
    }

    #[test]
    fn peak_and_average_per_session() {
        let out = run(&[
            progress("a", "V1", Some("s1"), Some(10.0)),
            progress("b", "V1", Some("s1"), Some(20.0)),
            progress("c", "V1", Some("s2"), Some(7.0)),
        ]);
        assert_eq!(out.len(), 2);
        let s1 = find(&out, "V1", "s1");
        assert_eq!(s1["peak_power_kw"], 20.0);
        assert_eq!(s1["avg_power_kw"], 15.0);
        assert_eq!(find(&out, "V1", "s2")["avg_power_kw"], 7.0);
    }

    #[test]
    fn resent_events_count_once() {
        let out = run(&[
            progress("a", "V1", Some("s1"), Some(10.0)),
            progress("a", "V1", Some("s1"), Some(10.0)),
            progress("b", "V1", Some("s1"), Some(40.0)),
        ]);
        assert_eq!(out[0]["avg_power_kw"], 25.0);
    }

    #[test]
    fn same_session_id_on_two_vins_is_two_records() {
        let out = run(&[
            progress("a", "V1", Some("s1"), Some(10.0)),
            progress("b", "V2", Some("s1"), Some(30.0)),
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(find(&out, "V2", "s1")["peak_power_kw"], 30.0);
    }

    #[test]
    fn null_power_is_ignored_and_all_null_session_has_no_power() {
        let out = run(&[
            progress("a", "V1", Some("s1"), None),
            progress("b", "V1", Some("s1"), Some(12.0)),
            progress("c", "V1", Some("s2"), None),
        ]);
        assert_eq!(find(&out, "V1", "s1")["avg_power_kw"], 12.0);
        let s2 = find(&out, "V1", "s2");
        assert!(s2.get("peak_power_kw").is_none() && s2.get("avg_power_kw").is_none());
    }

    #[test]
    fn null_session_id_is_its_own_group() {
        let out = run(&[progress("a", "V1", None, Some(5.0))]);
        assert_eq!(out.len(), 1);
        assert!(out[0].get("session_id").is_none());
    }

    #[test]
    fn other_event_types_are_ignored() {
        let mut e = progress("a", "V1", Some("s1"), Some(5.0));
        e.kind = "charging_end".into();
        assert!(run(&[e]).is_empty());
    }

    #[test]
    fn average_never_exceeds_peak() {
        let events: Vec<Event> = [3.0, 150.0, 75.5, 0.1]
            .iter()
            .enumerate()
            .map(|(i, kw)| progress(&i.to_string(), "V1", Some("s1"), Some(*kw)))
            .collect();
        let out = run(&events);
        assert!(out[0]["avg_power_kw"].as_f64() <= out[0]["peak_power_kw"].as_f64());
    }
}
