mod new;
mod query;
mod register;

use self::new::NewEventMessage;
use self::query::QueryMessage;
use self::register::RegisterMessage;

use err::{ErrorKind, Result};
use failure::ResultExt;

use std::str::from_utf8;
use serde_json::{from_str, Value};

use rdkafka::client::EmptyContext;
use rdkafka::producer::FutureProducer;
use websocket::message::OwnedMessage;

pub enum Message {
    NewEvent(NewEventMessage),
    Register(RegisterMessage),
    Query(QueryMessage),
}

impl Message {
    pub fn process(&self, addr: String, producer: FutureProducer<EmptyContext>,
                   topic: String) -> Result<()> {
        match *self {
            Message::NewEvent(ref e) => e.process(addr, producer, topic),
            Message::Register(ref e) => e.process(addr, producer, topic),
            Message::Query(ref e) => e.process(addr, producer, topic),
        }
    }
}

pub trait MessageContents {
    fn process(&self, addr: String, producer: FutureProducer<EmptyContext>,
               topic: String) -> Result<()>;
}

/// Process a message that is received from a WebSocket connection.
pub fn parse_message(incoming_message: OwnedMessage) -> Result<Message> {
    // Process the message we just got by forwarding on to Kafka.
    let converted_message = match incoming_message {
        OwnedMessage::Binary(bytes) => Ok(from_utf8(&bytes).context(
                ErrorKind::InvalidUtf8Bytes)?.to_string()),
        OwnedMessage::Text(message) => Ok(message),
        _ => Err(ErrorKind::InvalidWebsocketMessageType)
    }?;

    let parsed_message: Value = from_str(&converted_message).context(
        ErrorKind::IncomingJsonParse)?;
    let message_type = parsed_message["message_type"].as_str().ok_or(
        ErrorKind::NoMessageTypeProvided)?;

    match message_type {
        "query" => {
            let event: QueryMessage = from_str(&converted_message).context(
                ErrorKind::QueryMessageParse)?;
            Ok(Message::Query(event))
        },
        "new" => {
            let event: NewEventMessage = from_str(&converted_message).context(
                ErrorKind::NewEventMessageParse)?;
            Ok(Message::NewEvent(event))
        },
        "register" => {
            let event: RegisterMessage = from_str(&converted_message).context(
                ErrorKind::RegisterMessageParse)?;
            Ok(Message::Register(event))
        },
        _ => {
            error!("Invalid message type: {:?}", message_type);
            Err(ErrorKind::InvalidMessageType).map_err(From::from)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_invalid_websocket_message_type() {
        let ping = OwnedMessage::Ping(vec![]);
        let pong = OwnedMessage::Pong(vec![]);
        let close = OwnedMessage::Close(None);

        let parsed_ping = parse_message(ping);
        let parsed_pong = parse_message(pong);
        let parsed_close = parse_message(close);

        assert!(parsed_ping.is_err());
        if let Err(e) = parsed_ping {
            assert_eq!(e.kind(), ErrorKind::InvalidWebsocketMessageType);
        }

        assert!(parsed_pong.is_err());
        if let Err(e) = parsed_pong {
            assert_eq!(e.kind(), ErrorKind::InvalidWebsocketMessageType);
        }

        assert!(parsed_close.is_err());
        if let Err(e) = parsed_close {
            assert_eq!(e.kind(), ErrorKind::InvalidWebsocketMessageType);
        }
    }

    #[test]
    fn parse_invalid_json_string() {
        let data = r#"{
                        "error": "trailing comma",
                   }"#;
        let message = OwnedMessage::Text(data.to_string());
        let parsed = parse_message(message);

        assert!(parsed.is_err());
        if let Err(e) = parsed {
            assert_eq!(e.kind(), ErrorKind::IncomingJsonParse);
        }
    }

    #[test]
    fn parse_invalid_json_bytes() {
        let data = r#"{
                        "error": "trailing comma",
                   }"#;
        let message = OwnedMessage::Binary(data.as_bytes().to_vec());
        let parsed = parse_message(message);

        assert!(parsed.is_err());
        if let Err(e) = parsed {
            assert_eq!(e.kind(), ErrorKind::IncomingJsonParse);
        }
    }

    #[test]
    fn parse_no_message_type_string() {
        let data = r#"{
                        "data": "arbitrary"
                   }"#;
        let message = OwnedMessage::Text(data.to_string());
        let parsed = parse_message(message);

        assert!(parsed.is_err());
        if let Err(e) = parsed {
            assert_eq!(e.kind(), ErrorKind::NoMessageTypeProvided);
        }
    }

    #[test]
    fn parse_no_message_type_bytes() {
        let data = r#"{
                        "data": "arbitrary"
                   }"#;
        let message = OwnedMessage::Binary(data.as_bytes().to_vec());
        let parsed = parse_message(message);

        assert!(parsed.is_err());
        if let Err(e) = parsed {
            assert_eq!(e.kind(), ErrorKind::NoMessageTypeProvided);
        }
    }

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
        let message = OwnedMessage::Text(data.to_string());
        let parsed = parse_message(message);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            if let Message::NewEvent(message) = message {
                assert_eq!(message.events[0].event_type, "deposit");
                assert_eq!(message.events[0].data["account"], 837);
                assert_eq!(message.events[0].data["amount"], 3);
                assert_eq!(message.events[1].event_type, "withdrawal");
                assert_eq!(message.events[1].data["account"], 2837);
                assert_eq!(message.events[1].data["amount"], 5);
            } else {
                assert!(false);
            }
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
        let message = OwnedMessage::Text(data.to_string());
        let parsed = parse_message(message);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            if let Message::Register(message) = message {
                assert_eq!(message.event_types[0], "deposit");
                assert_eq!(message.event_types[1], "withdrawal");
            } else {
                assert!(false);
            }
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
        let message = OwnedMessage::Text(data.to_string());
        let parsed = parse_message(message);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            if let Message::Query(message) = message {
                assert_eq!(message.event_types[0], "deposit");
                assert_eq!(message.event_types[1], "withdrawal");
                assert_eq!(message.since, "2010-06-09T15:20:00-07:00");
            } else {
                assert!(false);
            }
        }
    }

    #[test]
    fn parse_invalid_message_type() {
        let data = r#"{
                        "message_type": "dog"
                   }"#;
        let message = OwnedMessage::Text(data.to_string());
        let parsed = parse_message(message);

        assert!(parsed.is_err());
        if let Err(e) = parsed {
            assert_eq!(e.kind(), ErrorKind::InvalidMessageType);
        }
    }
}
