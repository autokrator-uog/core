#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Register {
    pub client_type: String,
    pub event_types: Vec<String>,
    pub message_type: String,
}

mod tests {
    #[test]
    fn parse_register_message_type() {
        let data = r#"{
                        "message_type": "register",
                        "event_types": [
                            "deposit",
                            "withdrawal"
                        ],
                        "client_type": "transaction"
                   }"#;
        let parsed: Result<Register, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.message_type, "register");
            assert_eq!(message.event_types[0], "deposit");
            assert_eq!(message.event_types[1], "withdrawal");
            assert_eq!(message.client_type, "transaction");
        }
    }
}
