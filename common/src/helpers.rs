use std::str::from_utf8;

use failure::{Error, ResultExt};
use websocket::message::OwnedMessage;

use error::ErrorKind;

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
