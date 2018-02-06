use serde_json::Value;

use schemas::consistency::{
    Consistency,
    ConsistencyKey,
    ConsistencyValue,
    HasConsistency
};

#[derive(Serialize, Deserialize, Clone)]
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

impl HasConsistency for Event {
    fn consistency_key(&self) -> ConsistencyKey { self.consistency.key.clone() }
    fn consistency_value(&self) -> ConsistencyValue { self.consistency.value.clone() }
}
