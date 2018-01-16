/// This module contains the schemas for all the outgoing (to Kafka) messages we get.
/// We don't include the `message_type` field in these. Top-level structs are appended with
/// 'Message'.
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone)]
pub struct EventMessage {
    pub event_type: String,
    pub timestamp: String,
    pub session_id: usize,
    pub correlation_id: usize,
    pub sender: String,
    pub data: Value,
}
