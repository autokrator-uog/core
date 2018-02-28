use std::net::SocketAddr;

use actix::{Context, Handler, ResponseType};
use common::schemas::Event;
use failure::{Error, ResultExt};
use serde_json::from_str;

use bus::Bus;
use error::ErrorKind;

/// The `Acknowledgement` message is sent to the Bus when a client acknowledges receipt and
/// processing of a message.
#[derive(Clone)]
pub struct Acknowledgement {
    pub message: String,
    pub addr: SocketAddr,
}

impl ResponseType for Acknowledgement {
    type Item = ();
    type Error = ();
}

impl Bus {
    fn process_acknowledgement(&mut self, message: Acknowledgement) -> Result<(), Error> {
        let parsed: Event = from_str(&message.message).context(ErrorKind::ParseAcknowledgement)?;
        match self.sessions.get_mut(&message.addr) {
            Some(details) => {
                if details.unacknowledged_events.remove(&parsed) {
                    info!("successfully removed event from unacknowledged events: client='{}'",
                          message.addr);
                } else {
                    warn!("attempt to remove unacknowledged event that does not exist");
                }

                Ok(())
            },
            None => return Err(Error::from(ErrorKind::SessionNotInHashMap)),
        }
    }
}

impl Handler<Acknowledgement> for Bus {
    type Result = ();

    fn handle(&mut self, message: Acknowledgement, _: &mut Context<Self>) {
        debug!("received 'acknowledgement' signal: client='{}'", message.addr);
        if let Err(e) = self.process_acknowledgement(message) {
            error!("processing unacknowledgement: error='{}'", e);
        }
    }
}
