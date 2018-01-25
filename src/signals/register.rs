use std::net::SocketAddr;

use actix::{Address, Context, Handler, ResponseType};
use failure::{Error, ResultExt};
use serde_json::{from_str, to_string_pretty};

use bus::{Bus, RegisteredTypes};
use error::ErrorKind;
use schemas;
use session::Session;
use signals::SendToClient;

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
    pub fn register(&mut self, message: Register) -> Result<(), Error> {
        let (addr, socket) = message.sender;

        let parsed: schemas::incoming::RegisterMessage = from_str(&message.message).context(
            ErrorKind::ParseNewEventMessage)?;
        info!("parsed register message: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);

        let response = schemas::outgoing::Registration {
            message_type: "registration".to_string(),
            event_types: parsed.event_types.clone(),
        };

        match self.sessions.get_mut(&socket) {
            Some(details) => {
                if parsed.event_types.len() == 1 && parsed.event_types[0] == "*" {
                    info!("updated register types for client: client=`{:?}` types=all", socket);
                    details.registered_types = RegisteredTypes::All;
                }
                else {
                    info!("updated register types for client: client=`{:?}` types=`{:?}`",
                          socket, parsed.event_types);
                    details.registered_types = RegisteredTypes::Some(parsed.event_types);
                }

                info!("marking client as registered: client=`{:?}`", socket);
                details.is_registered = true;
            },
            None => {
                error!("client is not present in sessions. this is a bug.");
                return Err(Error::from(ErrorKind::SessionNotInHashMap));
            },
        }

        info!("sending receipt to the client");
        addr.send(SendToClient(response));
        Ok(())
    }
}


impl Handler<Register> for Bus {
    type Result = ();

    fn handle(&mut self, message: Register, _: &mut Context<Self>) {
        if let Err(e) = self.register(message) {
            error!("processing new event: error='{}'", e);
        }
    }
}
