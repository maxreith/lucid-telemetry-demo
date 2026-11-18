pub mod event;

pub use event::{dedup, of_type, read_events, Event};

/// A job reads all recorded events and returns its output records.
pub type Job = fn(&[Event]) -> Vec<serde_json::Value>;

pub mod jobs {
    include!(concat!(env!("OUT_DIR"), "/jobs.rs"));
}
