//! Port of scala/.../jobs/EcuFaultSummary.scala: faults and distinct affected vehicles per ECU.
//! Events without an ECU form their own group with `ecu` null, as Spark's groupBy does.
use crate::{dedup, of_type, Event};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

pub fn run(events: &[Event]) -> Vec<Value> {
    let mut by_ecu: BTreeMap<Option<&str>, (u64, HashSet<&str>)> = BTreeMap::new();
    for e in of_type(dedup(events), "diagnostic") {
        let (faults, vins) = by_ecu.entry(e.str("ecu")).or_default();
        *faults += 1;
        vins.insert(e.vin.as_str());
    }
    by_ecu
        .into_iter()
        .map(|(ecu, (faults, vins))| {
            json!({"ecu": ecu, "faults": faults, "affected_vins": vins.len()})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fault(event_id: &str, vin: &str, ecu: Option<&str>) -> Event {
        let payload = match ecu {
            Some(ecu) => json!({"code": "P0AA6", "severity": "critical", "ecu": ecu}),
            None => json!({"code": "P0AA6", "severity": "critical"}),
        };
        serde_json::from_value(json!({
            "event_id": event_id, "vin": vin, "type": "diagnostic", "event_ts": 1, "received_ts": 1,
            "fw_version": "1.0", "payload": payload
        }))
        .unwrap()
    }

    fn row(out: &[Value], ecu: Value) -> &Value {
        out.iter().find(|r| r["ecu"] == ecu).unwrap()
    }

    #[test]
    fn resent_faults_are_counted_once() {
        let out = run(&[fault("a", "V1", Some("BMS")), fault("a", "V1", Some("BMS"))]);
        assert_eq!(out, vec![json!({"ecu": "BMS", "faults": 1, "affected_vins": 1})]);
    }

    #[test]
    fn counts_faults_and_distinct_vins_per_ecu() {
        let out = run(&[
            fault("a", "V1", Some("BMS")),
            fault("b", "V1", Some("BMS")),
            fault("c", "V2", Some("BMS")),
            fault("d", "V2", Some("INV")),
        ]);
        assert_eq!(row(&out, json!("BMS"))["faults"], 3);
        assert_eq!(row(&out, json!("BMS"))["affected_vins"], 2);
        assert_eq!(row(&out, json!("INV"))["faults"], 1);
    }

    #[test]
    fn missing_ecu_is_its_own_group() {
        let out = run(&[fault("a", "V1", None), fault("b", "V1", Some("BMS"))]);
        assert_eq!(out.len(), 2);
        assert_eq!(row(&out, Value::Null)["faults"], 1);
    }

    #[test]
    fn ignores_other_event_types() {
        let mut e = fault("a", "V1", Some("BMS"));
        e.kind = "battery_status".into();
        assert!(run(&[e]).is_empty());
    }

    #[test]
    fn affected_vins_never_exceed_faults() {
        let out = run(&[
            fault("a", "V1", Some("BMS")),
            fault("b", "V2", Some("BMS")),
            fault("c", "V2", Some("GW")),
        ]);
        for r in &out {
            assert!(r["affected_vins"].as_u64() <= r["faults"].as_u64());
        }
    }
}
