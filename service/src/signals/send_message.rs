use actix::{Context, Handler, ResponseType};
use failure::{Error, ResultExt};
use serde::Serialize;
use serde_json::{to_string, to_string_pretty};
use websocket::OwnedMessage;

use client::Client;
use error::ErrorKind;

/// The `SendMessage` signal is sent from the interpreter to the client when a new message needs to
/// be sent to the event bus.
pub struct SendMessage<T: Serialize + Send>(pub T);

impl<T: Send> ResponseType for SendMessage<T>
    where T: Serialize
{
    type Item = ();
    type Error = ();
}

impl Client {
    pub fn send_message<T: Serialize>(&mut self, message: T) -> Result<(), Error> {
        let serialized = to_string(&message).context(
            ErrorKind::SerializeJsonForSending)?;
        let pretty_serialized = to_string_pretty(&message).context(
            ErrorKind::SerializeJsonForSending)?;

        info!("sending message: message=\n{}", pretty_serialized);
        Ok(self.framed.send(OwnedMessage::Text(serialized)))
    }
}

impl<T: Send> Handler<SendMessage<T>> for Client
    where T: Serialize
{
    type Result = ();

    fn handle(&mut self, message: SendMessage<T>, _: &mut Context<Self>) {
        info!("received send message signal from interpreter");
        if let Err(e) = self.send_message(message.0) {
            error!("unable to send to message on websockets: error='{}'", e);
        }
    }
}
