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
