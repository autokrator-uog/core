use std::str::from_utf8;

use failure::{Error, ResultExt};
use serde::Serialize;
use serde_json::to_string;
use sha1::Sha1;
use websocket::message::OwnedMessage;

use error::ErrorKind;

pub fn hash_json<T: Serialize>(input: &T) -> Result<String, Error> {
    let json = to_string(input).context(ErrorKind::SerializeJsonForHashing)?;

    let mut hasher = Sha1::new();
    hasher.update(json.as_bytes());
    Ok(hasher.digest().to_string())
}

/// Returns the contents of a message from a Websockets message.
pub fn websocket_message_contents(message: OwnedMessage) -> Result<String, Error> {
    match message {
        OwnedMessage::Text(m) => Ok(m),
        OwnedMessage::Binary(b) => {
            Ok(from_utf8(&b).context(ErrorKind::ParseBytesAsUtf8)?.to_string())
        },
        OwnedMessage::Close(_) => {
            // We don't stop the actor here as this causes issues with Actix. The stream
            // closes itself in a moment.
            info!("client has disconnected, stream will close and session will be removed \
                   momentarily");
            Err(Error::from(ErrorKind::InvalidWebsocketMessageType))
        },
        OwnedMessage::Ping(_) => {
            Err(Error::from(ErrorKind::InvalidWebsocketMessageType))
        }
        OwnedMessage::Pong(_) => {
            Err(Error::from(ErrorKind::InvalidWebsocketMessageType))
        }
    }
}
