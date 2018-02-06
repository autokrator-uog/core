use serde_json::Value;

use schemas::consistency::Consistency;

#[derive(Serialize, Deserialize, Clone)]
pub struct NewEvents {
    pub message_type: String,
    pub events: Vec<NewEvent>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NewEvent {
    pub consistency: Consistency,
    pub correlation_id: usize,
    pub data: Value,
    pub event_type: String,
}

mod tests {
    #[test]
    fn parse_new_message_type() {
        let data = r#"{
                        "message_type": "new",
                        "events": [
                            {
                                "event_type": "deposit",
                                "correlation_id": 94859829321,
                                "data": {
                                    "account": 837,
                                    "amount": 3
                                },
                                "consistency": {
                                    "key": "testkey",
                                    "value": "*"
                                }
                            },
                            {
                                "event_type": "withdrawal",
                                "correlation_id": 94859829321,
                                "data": {
                                    "account": 2837,
                                    "amount": 5
                                },
                                "consistency": {
                                    "key": "testkey",
                                    "value": 123456
                                }
                            }
                        ]
                   }"#;
        let parsed: Result<NewEvents, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.message_type, "new");
            assert_eq!(message.events[0].event_type, "deposit");
            assert_eq!(message.events[0].correlation_id, 94859829321);
            assert_eq!(message.events[0].data["account"], 837);
            assert_eq!(message.events[0].data["amount"], 3);
            assert_eq!(message.events[0].consistency.key, "testkey");
            assert_eq!(message.events[1].event_type, "withdrawal");
            assert_eq!(message.events[1].correlation_id, 94859829321);
            assert_eq!(message.events[1].data["account"], 2837);
            assert_eq!(message.events[1].data["amount"], 5);
            assert_eq!(message.events[1].consistency.key, "testkey");
        }
    }
}
