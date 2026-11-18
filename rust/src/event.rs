use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::{fs, io, path::Path};

/// One vehicle-to-cloud telemetry event, as recorded from Kafka.
#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub vin: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub event_ts: i64,
    pub received_ts: i64,
    pub fw_version: String,
    pub payload: Value,
}

impl Event {
    pub fn f64(&self, field: &str) -> Option<f64> {
        self.payload.get(field)?.as_f64()
    }

    pub fn str(&self, field: &str) -> Option<&str> {
        self.payload.get(field)?.as_str()
    }
}

pub fn read_events(path: &Path) -> io::Result<Vec<Event>> {
    fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(io::Error::other))
        .collect()
}

/// Cars resend events when an upload is not acknowledged; a resent copy keeps its event_id.
pub fn dedup(events: &[Event]) -> Vec<&Event> {
    let mut seen = HashSet::new();
    events
        .iter()
        .filter(|e| seen.insert(e.event_id.as_str()))
        .collect()
}

pub fn of_type<'a>(events: impl IntoIterator<Item = &'a Event>, kind: &str) -> Vec<&'a Event> {
    events.into_iter().filter(|e| e.kind == kind).collect()
}
