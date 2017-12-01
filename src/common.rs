use serde_json::value::Value;

pub type EventContents = Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub message_type: String,
    pub timestamp: String,
    pub addr: String,
    pub event_type: String,
    pub data: EventContents,
}
