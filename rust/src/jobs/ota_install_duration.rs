//! Port of scala/.../jobs/OtaInstallDuration.scala: seconds from install start to install success per vehicle
//! and campaign.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Default)]
struct Install {
    installing_ts: Option<i64>,
    installed_ts: Option<i64>,
}

fn keep_min(slot: &mut Option<i64>, ts: i64) {
    *slot = Some(slot.map_or(ts, |cur| cur.min(ts)));
}

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut installs: BTreeMap<(&str, Option<&str>), Install> = BTreeMap::new();
    for e in of_type(dedup(events), "ota_status") {
        let install = installs
            .entry((e.vin.as_str(), e.str("campaign_id")))
            .or_default();
        match e.str("state") {
            Some("installing") => keep_min(&mut install.installing_ts, e.event_ts),
            Some("installed") => keep_min(&mut install.installed_ts, e.event_ts),
            _ => {}
        }
    }
    installs
        .into_iter()
        .filter_map(|((vin, campaign_id), install)| {
            let (start, end) = (install.installing_ts?, install.installed_ts?);
            Some(json!({
                "vin": vin,
                "campaign_id": campaign_id,
                "duration_s": (end - start) as f64 / 1000.0,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn status(event_id: &str, vin: &str, campaign: Option<&str>, state: &str, ts: i64) -> Event {
        let mut payload = json!({"version": "2.6.0", "state": state});
        if let Some(c) = campaign {
            payload["campaign_id"] = json!(c);
        }
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "ota_status", "event_ts": ts, "received_ts": ts,
            "fw_version": "2.5.0", "payload": payload
        }))
        .unwrap()
    }

    const C: Option<&str> = Some("OTA-A");

    #[test]
    fn duration_from_first_installing_to_first_installed() {
        let out = run(&[
            status("a", "V1", C, "downloading", 0),
            status("b", "V1", C, "installing", 2_000),
            status("c", "V1", C, "installing", 5_000),
            status("d", "V1", C, "installed", 9_500),
            status("e", "V1", C, "installed", 20_000),
        ]);
        assert_eq!(out, vec![json!({"vin": "V1", "campaign_id": "OTA-A", "duration_s": 7.5})]);
    }

    #[test]
    fn needs_both_installing_and_installed() {
        let out = run(&[
            status("a", "V1", C, "installing", 0),
            status("b", "V1", C, "failed", 1_000),
            status("c", "V2", C, "installed", 1_000),
        ]);
        assert!(out.is_empty());
    }

    #[test]
    fn groups_by_vin_and_campaign_and_ignores_resent_copies() {
        let out = run(&[
            status("a", "V1", C, "installing", 0),
            status("a", "V1", C, "installing", 0),
            status("b", "V1", C, "installed", 1_000),
            status("c", "V1", Some("OTA-B"), "installing", 0),
            status("d", "V1", Some("OTA-B"), "installed", 3_000),
            status("e", "V2", None, "installing", 0),
            status("f", "V2", None, "installed", 2_000),
        ]);
        assert_eq!(out.len(), 3);
        assert!(out.contains(&json!({"vin": "V2", "campaign_id": null, "duration_s": 2.0})));
    }

    #[test]
    fn durations_are_never_negative_on_recorded_data() {
        let Ok(events) = crate::read_events(std::path::Path::new("../data/events.jsonl")) else {
            return;
        };
        for r in run(&events) {
            assert!(r["duration_s"].as_f64().unwrap() >= 0.0, "{r}");
        }
    }
}
