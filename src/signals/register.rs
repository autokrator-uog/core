use std::net::SocketAddr;

use actix::{Actor, Address, Context, Handler, Response, ResponseType};
use chrono::Local;
use failure::{Error, ResultExt};
use serde::Serialize;
use serde_json::{from_str, to_string, to_string_pretty, Value};
use sha1::Sha1;

use bus::Bus;
use error::ErrorKind;
use messages::SendToClient;
use schemas;
use session::Session;

/// The `Register` message is sent to the Bus when a client wants to provide more information about
/// itself or limit event types it can receive.
pub struct Register {
    pub message: String,
    pub bus: Address<Bus>,
    pub sender: (Address<Session>, SocketAddr),
}

impl ResponseType for Register {
    type Item = ();
    type Error = ();
}

impl Bus {
   
    pub fn register_message(&mut self, message: Register) -> Result<(), Error> {
        let (addr, socket) = message.sender;

        let parsed: schemas::incoming::RegisterMessage = from_str(&message.message).context(
            ErrorKind::ParseNewEventMessage)?;
        info!("parsed new event message: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);

        let mut receipt = schemas::outgoing::Registration {
            message_type: "registration".to_string(),
	        event_types: parsed.event_types,
        };

	    match self.sessions.get(socket) {
	        Some(details) => {
                if event_types.len() == 1 && event.types[0] == "*" {
                    details.registered_types = RegisteredTypes::All;
                }
                else {
                    details.registered_types = RegisteredTypes::Some(parsed.event_types);
                }
            },

            None => {
                error!("Client is not present in HashMap. This is a bug.");
                return Err(Error::from(ErrorKind::SessionNotInHashMap));
            },
        }
        

        info!("sending receipt to the client");
        addr.send(SendToClient(receipt));

        Ok(())
    }
}


impl Handler<Register> for Bus {
    fn handle(&mut self, _message: Register, _: &mut Context<Self>) -> Response<Self, Register> {
        Self::empty()
    }
}
