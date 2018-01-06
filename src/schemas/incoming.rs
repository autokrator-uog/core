/// This module contains the schemas for all the incoming messages we get. We don't include
/// the `message_type` field in these. Top-level structs are appended with 'Message'.
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone)]
pub struct NewEventMessage {
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Event {
    pub event_type: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QueryMessage {
    pub event_types: Vec<String>,
    pub since: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RegisterMessage {
    pub event_types: Vec<String>,
}

#[cfg(test)]
mod event_tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn parse_new_message_type() {
        let data = r#"{
                        "message_type": "new",
                        "events": [
                            {
                                "event_type": "deposit",
                                "data": {
                                    "account": 837,
                                    "amount": 3
                                }
                            },
                            {
                                "event_type": "withdrawal",
                                "data": {
                                    "account": 2837,
                                    "amount": 5
                                }
                            }
                        ]
                   }"#;
        let parsed: Result<NewEventMessage, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.events[0].event_type, "deposit");
            assert_eq!(message.events[0].data["account"], 837);
            assert_eq!(message.events[0].data["amount"], 3);
            assert_eq!(message.events[1].event_type, "withdrawal");
            assert_eq!(message.events[1].data["account"], 2837);
            assert_eq!(message.events[1].data["amount"], 5);
        }
    }

    #[test]
    fn parse_register_message_type() {
        let data = r#"{
                        "message_type": "register",
                        "event_types": [
                            "deposit",
                            "withdrawal"
                        ]
                   }"#;
        let parsed: Result<RegisterMessage, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.event_types[0], "deposit");
            assert_eq!(message.event_types[1], "withdrawal");
        }
    }

    #[test]
    fn parse_query_message_type() {
        let data = r#"{
                        "message_type": "query",
                        "event_types": [
                            "deposit",
                            "withdrawal"
                        ],
                        "since": "2010-06-09T15:20:00-07:00"
                   }"#;
        let parsed: Result<QueryMessage, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.event_types[0], "deposit");
            assert_eq!(message.event_types[1], "withdrawal");
            assert_eq!(message.since, "2010-06-09T15:20:00-07:00");
        }
    }
}
