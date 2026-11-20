//! Port of scala/.../jobs/OtaCampaignSummary.scala: vehicles per OTA campaign by their latest state.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Default)]
struct Summary {
    vehicles: i64,
    installed: i64,
    failed: i64,
    in_progress: i64,
}

/// Keeps the most recent event per key by event time; ties are broken by the larger event_id.
fn latest_by<K: Ord>(events: Vec<&Event>, key: impl Fn(&Event) -> K) -> Vec<&Event> {
    let mut latest: BTreeMap<K, &Event> = BTreeMap::new();
    for e in events {
        latest
            .entry(key(e))
            .and_modify(|cur| {
                if (e.event_ts, e.event_id.as_str()) > (cur.event_ts, cur.event_id.as_str()) {
                    *cur = e;
                }
            })
            .or_insert(e);
    }
    latest.into_values().collect()
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let ota = of_type(dedup(events), "ota_status");
    let latest = latest_by(ota, |e| (e.vin.clone(), e.str("campaign_id").map(str::to_owned)));

    let mut campaigns: BTreeMap<Option<&str>, Summary> = BTreeMap::new();
    for e in latest {
        let s = campaigns.entry(e.str("campaign_id")).or_default();
        s.vehicles += 1;
        match e.str("state") {
            Some("installed") => s.installed += 1,
            Some("failed") => s.failed += 1,
            Some("downloading" | "installing") => s.in_progress += 1,
            _ => {}
        }
    }
    campaigns
        .into_iter()
        .map(|(campaign_id, s)| {
            json!({
                "campaign_id": campaign_id,
                "vehicles": s.vehicles,
                "installed": s.installed,
                "failed": s.failed,
                "in_progress": s.in_progress,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(event_id: &str, vin: &str, ts: i64, campaign: Option<&str>, state: Option<&str>) -> Event {
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "ota_status", "event_ts": ts, "received_ts": ts,
            "fw_version": "2.5.0", "payload": {"campaign_id": campaign, "version": "2.6.0", "state": state}
        }))
        .unwrap()
    }

    fn by_campaign(out: &[Value], campaign: Value) -> &Value {
        out.iter().find(|r| r["campaign_id"] == campaign).unwrap()
    }

    #[test]
    fn counts_each_vehicle_once_by_latest_state() {
        let out = run(&[
            status("a", "V1", 1, Some("C"), Some("downloading")),
            status("b", "V1", 3, Some("C"), Some("installed")),
            // Late arrival of an older state must not win.
            status("c", "V1", 2, Some("C"), Some("installing")),
            status("d", "V2", 1, Some("C"), Some("failed")),
        ]);
        assert_eq!(
            out,
            vec![json!({"campaign_id": "C", "vehicles": 2, "installed": 1, "failed": 1, "in_progress": 0})]
        );
    }

    #[test]
    fn resent_events_are_counted_once() {
        let e = status("a", "V1", 1, Some("C"), Some("installing"));
        let out = run(&[e.clone(), e]);
        assert_eq!(out[0]["vehicles"], 1);
        assert_eq!(out[0]["in_progress"], 1);
    }

    #[test]
    fn ties_on_event_ts_go_to_larger_event_id() {
        let out = run(&[
            status("b", "V1", 5, Some("C"), Some("failed")),
            status("a", "V1", 5, Some("C"), Some("installed")),
        ]);
        assert_eq!(out[0]["failed"], 1);
        assert_eq!(out[0]["installed"], 0);
    }

    #[test]
    fn null_campaign_and_unknown_state_are_kept() {
        let out = run(&[
            status("a", "V1", 1, None, Some("installed")),
            status("b", "V2", 1, Some("C"), None),
            status("c", "V3", 1, Some("C"), Some("queued")),
        ]);
        assert_eq!(by_campaign(&out, Value::Null)["installed"], 1);
        let c = by_campaign(&out, json!("C"));
        assert_eq!(c["vehicles"], 2);
        assert_eq!(c["installed"].as_i64().unwrap() + c["failed"].as_i64().unwrap(), 0);
        assert_eq!(c["in_progress"], 0);
    }

    #[test]
    fn state_counts_never_exceed_vehicles() {
        let out = run(&[
            status("a", "V1", 1, Some("C"), Some("installed")),
            status("b", "V1", 2, Some("D"), Some("downloading")),
            status("c", "V2", 1, Some("C"), Some("failed")),
        ]);
        for r in &out {
            let parts: i64 = ["installed", "failed", "in_progress"].iter().map(|k| r[k].as_i64().unwrap()).sum();
            assert!(parts <= r["vehicles"].as_i64().unwrap());
        }
    }
}
