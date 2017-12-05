use common::Event;

use err::{ErrorKind, Result};
use failure::ResultExt;

use serde_json::{from_str, to_string};

/// Parse a message from Kafka.
///
/// If `None` is returned, it will not be sent to any WebSocket clients.
pub fn parse_message(raw_message: String) -> Result<Event> {
    info!("Parsing message from Kafka: {:?}", raw_message);
    from_str(&raw_message).context(ErrorKind::KafkaJsonParse).map_err(From::from)
}

/// Decide whether a message should be sent to a client.
///
/// If `None` is returned, it will not be sent to any WebSocket clients.
pub fn process_event(mut event: Event, to: &str) -> Result<String> {
    // Override type as now we're sending an event, not receiving a new one
    event.message_type = "event".to_string();

    info!("Sending message to {:?}: {:?}", to, event.data);
    to_string(&event).context(ErrorKind::ConsumerProcessingFailure).map_err(From::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_message() {
        let data = r#"{
                        "message_type": "event",
                        "timestamp": "Wed, 9 Jun 2010 22:20:00 UTC",
                        "addr": "V4(127.0.0.1:45938)",
                        "event_type": "deposit",
                        "data": {
                            "account": 3847,
                            "amount": 3
                        }
                  }"#;

        let parsed = parse_message(data.to_string());
        assert!(parsed.is_ok());

        if let Ok(message) = parsed {
            assert_eq!(message.message_type, "event");
            assert_eq!(message.timestamp, "Wed, 9 Jun 2010 22:20:00 UTC");
            assert_eq!(message.addr, "V4(127.0.0.1:45938)");
            assert_eq!(message.event_type, "deposit");
            assert_eq!(message.data["account"], 3847);
            assert_eq!(message.data["amount"], 3);
        }
    }

    #[test]
    fn parse_invalid_message() {
        let data = r#"{
                        "name": "John Doe",
                        "age": 43,
                        "phones": [
                              "+44 1234567",
                              "+44 2345678"
                        ]
                  }"#;
        let parsed = parse_message(data.to_string());

        assert!(parsed.is_err());
        if let Err(e) = parsed {
            assert_eq!(e.kind(), ErrorKind::KafkaJsonParse);
        }
    }
}
