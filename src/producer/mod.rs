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

pub trait Message {
    fn process(&self, addr: String, producer: FutureProducer<EmptyContext>, topic: String);
}

/// Process a message that is received from a WebSocket connection.
pub fn parse_message(incoming_message: OwnedMessage) -> Result<Box<Message>> {
    // Process the message we just got by forwarding on to Kafka.
    let converted_message = match incoming_message {
        OwnedMessage::Binary(bytes) => Ok(from_utf8(&bytes).context(
                ErrorKind::InvalidUtf8Bytes)?.to_string()),
        OwnedMessage::Text(message) => Ok(message),
        _ => Err(ErrorKind::InvalidWebsocketMessageType)
    }?;

    let parsed_message: Value = from_str(&converted_message).context(ErrorKind::JsonParse)?;
    let message_type = parsed_message["message_type"].as_str().ok_or(
        ErrorKind::NoMessageTypeProvided)?;

    match message_type {
        "query" => {
            let event: QueryMessage = from_str(&converted_message).context(
                ErrorKind::QueryMessageParse)?;
            Ok(Box::new(event) as Box<Message>)
        },
        "new" => {
            let event: NewEventMessage = from_str(&converted_message).context(
                ErrorKind::NewEventMessageParse)?;
            Ok(Box::new(event) as Box<Message>)
        },
        "register" => {
            let event: RegisterMessage = from_str(&converted_message).context(
                ErrorKind::RegisterMessageParse)?;
            Ok(Box::new(event) as Box<Message>)
        },
        _ => {
            error!("Invalid message type: {:?}", message_type);
            Err(ErrorKind::InvalidMessageType)?
        },
    }
}
