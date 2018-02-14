use std::net::SocketAddr;

use actix::{Context, Handler, ResponseType};
use common::schemas::Event;
use failure::{Error, ResultExt};
use serde_json::from_str;

use bus::Bus;
use error::ErrorKind;

/// The `Awknowledgement` message is sent to the Bus when a client awknowledges receipt and
/// processing of a message.
#[derive(Clone)]
pub struct Awknowledgement {
    pub message: String,
    pub addr: SocketAddr,
}

impl ResponseType for Awknowledgement {
    type Item = ();
    type Error = ();
}

impl Bus {
    fn process_awknowledgement(&mut self, message: Awknowledgement) -> Result<(), Error> {
        let parsed: Event = from_str(&message.message).context(ErrorKind::ParseAwknowledgement)?;
        match self.sessions.get_mut(&message.addr) {
            Some(details) => {
                if details.unawknowledged_events.remove(&parsed) {
                    info!("successfully removed event from unawknowledged events: client='{}'",
                          message.addr);
                } else {
                    warn!("attempt to remove unawknowledged event that does not exist");
                }

                Ok(())
            },
            None => return Err(Error::from(ErrorKind::SessionNotInHashMap)),
        }
    }
}

impl Handler<Awknowledgement> for Bus {
    type Result = ();

    fn handle(&mut self, message: Awknowledgement, _: &mut Context<Self>) {
        debug!("received 'awknowledgement' signal: client='{}'", message.addr);
        if let Err(e) = self.process_awknowledgement(message) {
            error!("processing awknowledgement: error='{}'", e);
        }
    }
}
