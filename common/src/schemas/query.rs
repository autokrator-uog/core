#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Query {
    pub message_type: String,
    pub event_types: Vec<String>,
    pub since: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

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
        let parsed: Result<Query, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.message_type, "query");
            assert_eq!(message.event_types[0], "deposit");
            assert_eq!(message.event_types[1], "withdrawal");
            assert_eq!(message.since, "2010-06-09T15:20:00-07:00");
        }
    }
}
