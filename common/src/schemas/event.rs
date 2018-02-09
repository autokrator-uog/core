use std::hash::{Hash, Hasher};

use serde_json::Value;

use schemas::consistency::Consistency;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Event {
    pub consistency: Consistency,
    pub correlation_id: u32,
    pub data: Value,
    pub event_type: String,
    // We don't want this field going to Kafka.
    pub message_type: Option<String>,
    pub sender: String,
    pub session_id: Option<usize>,
    pub timestamp: String,
    pub timestamp_raw: Option<i64>,
}

impl Eq for Event { }

impl Hash for Event {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.consistency.hash(state);
        self.correlation_id.hash(state);
        self.event_type.hash(state);
        self.message_type.hash(state);
        self.sender.hash(state);
        self.session_id.hash(state);
        self.timestamp.hash(state);
        self.timestamp_raw.hash(state);
    }
}
