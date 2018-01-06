/// This module contains the schemas for all the outgoing (to websockets) messages we get.
/// We include the `message_type` field in these. Top-level structs are appended with
/// 'Message'.
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone)]
pub struct ReceiptMessage {
    pub message_type: String,
    pub receipts: Vec<Receipt>,
    pub timestamp: String,
    pub sender: String,
}

#[derive(Serialize, Deserialize, Hash, Clone)]
pub struct Receipt {
    pub checksum: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EventMessage {
    pub message_type: String,
    pub event_type: String,
    pub timestamp: String,
    pub sender: String,
    pub data: Value,
}

#[cfg(test)]
mod event_tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn parse_valid_event() {
        let data = r#"{
                        "message_type": "event",
                        "timestamp": "Wed, 9 Jun 2010 22:20:00 UTC",
                        "sender": "V4(127.0.0.1:45938)",
                        "event_type": "deposit",
                        "data": {
                            "account": 3847,
                            "amount": 3
                        }
                   }"#;

        let parsed: Result<EventMessage, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.message_type, "event");
            assert_eq!(message.timestamp, "Wed, 9 Jun 2010 22:20:00 UTC");
            assert_eq!(message.sender, "V4(127.0.0.1:45938)");
            assert_eq!(message.event_type, "deposit");
            assert_eq!(message.data["account"], 3847);
            assert_eq!(message.data["amount"], 3);
        }
    }

    #[test]
    fn parse_invalid_event() {
        let data = r#"{
                        "name": "John Doe",
                        "age": 43,
                        "phones": [
                              "+44 1234567",
                              "+44 2345678"
                        ]
                   }"#;
        let parsed: Result<EventMessage, _> = from_str(data);
        assert!(parsed.is_err());
    }
}

