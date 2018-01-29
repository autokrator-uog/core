use actix::{Context, Handler, ResponseType};
use failure::{Error, ResultExt};
use serde::Serialize;
use serde_json::{to_string, to_string_pretty};
use websocket::message::OwnedMessage;

use error::ErrorKind;
use server::WsMessage;
use session::Session;

/// The `SendToClient` message is sent to a Session when a message needs to be sent to the client
/// managed by that session.
pub struct SendToClient<T: Serialize + Send>(pub T);

impl<T: Send> ResponseType for SendToClient<T>
    where T: Serialize
{
    type Item = ();
    type Error = ();
}

impl Session {
    pub fn send_message<T: Serialize>(&mut self, message: T) -> Result<(), Error> {
        let serialized = to_string(&message).context(
            ErrorKind::SerializeJsonForSending)?;
        let pretty_serialized = to_string_pretty(&message).context(
            ErrorKind::SerializeJsonForSending)?;

        info!("sending message: client='{}' message=\n{}", self.addr, pretty_serialized);
        Ok(self.framed.send(WsMessage(OwnedMessage::Text(serialized))))
    }
}

impl<T: Send> Handler<SendToClient<T>> for Session
    where T: Serialize
{
    type Result = ();

    fn handle(&mut self, message: SendToClient<T>,
              _ctx: &mut Context<Self>) {
        if let Err(e) = self.send_message(message.0) {
            error!("unable to send to message on websockets:\nclient='{}' error=\n{}",
                   self.addr, e);
        }
    }
}
