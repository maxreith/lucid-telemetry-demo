//! Port of scala/.../jobs/OtaLatestState.scala: latest OTA update state per vehicle and campaign, by event time.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut latest: HashMap<(&str, Option<&str>), &Event> = HashMap::new();
    for e in of_type(dedup(events), "ota_status") {
        latest
            .entry((e.vin.as_str(), e.str("campaign_id")))
            .and_modify(|cur| {
                if (e.event_ts, e.event_id.as_str()) > (cur.event_ts, cur.event_id.as_str()) {
                    *cur = e;
                }
            })
            .or_insert(e);
    }
    latest
        .into_iter()
        .map(|((vin, campaign_id), e)| {
            json!({
                "vin": vin,
                "campaign_id": campaign_id,
                "state": e.str("state"),
                "version": e.str("version"),
                "event_ts": e.event_ts,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ota(event_id: &str, vin: &str, campaign: Option<&str>, state: &str, ts: i64) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "ota_status", "event_ts": ts, "received_ts": ts + 5000,
            "fw_version": "2.5.0", "payload": {"campaign_id": campaign, "version": "2.6.0", "state": state}
        }))
        .unwrap()
    }

    fn state_of<'a>(out: &'a [Value], vin: &str, campaign: Option<&str>) -> Vec<&'a Value> {
        out.iter()
            .filter(|r| r["vin"] == vin && r["campaign_id"].as_str() == campaign)
            .map(|r| &r["state"])
            .collect()
    }

    #[test]
    fn keeps_latest_by_event_time_not_arrival_order() {
        let out = run(&[
            ota("b", "V1", Some("C1"), "installed", 300),
            ota("a", "V1", Some("C1"), "downloading", 100),
            ota("c", "V1", Some("C1"), "installing", 200),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["state"], "installed");
        assert_eq!(out[0]["event_ts"], 300);
        assert_eq!(out[0]["version"], "2.6.0");
    }

    #[test]
    fn ties_go_to_larger_event_id() {
        let out = run(&[
            ota("b", "V1", Some("C1"), "installed", 100),
            ota("a", "V1", Some("C1"), "failed", 100),
        ]);
        assert_eq!(state_of(&out, "V1", Some("C1")), ["installed"]);
    }

    #[test]
    fn one_record_per_vehicle_and_campaign() {
        let out = run(&[
            ota("a", "V1", Some("C1"), "installed", 100),
            ota("b", "V1", Some("C2"), "downloading", 100),
            ota("c", "V2", Some("C1"), "failed", 100),
            ota("d", "V1", None, "installing", 100),
            ota("e", "V1", None, "installed", 200),
        ]);
        assert_eq!(out.len(), 4);
        assert_eq!(state_of(&out, "V1", None), ["installed"]);
        assert!(out.iter().all(|r| r.get("event_id").is_none()));
    }

    #[test]
    fn resent_copies_and_other_types_are_ignored() {
        let mut other = ota("z", "V1", Some("C1"), "installed", 999);
        other.kind = "gps".into();
        let out = run(&[
            ota("a", "V1", Some("C1"), "installing", 100),
            ota("a", "V1", Some("C1"), "installing", 100),
            other,
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["state"], "installing");
    }
}
