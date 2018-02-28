use std::net::SocketAddr;

use actix::{Context, Handler, ResponseType};
use common::VecDequeExt;
use failure::Error;
use serde_json::to_string_pretty;

use bus::Bus;
use error::ErrorKind;

/// The `Disconnect` message is sent to the Bus when a client disconnects.
#[derive(Clone)]
pub struct Disconnect {
    pub addr: SocketAddr,
}

impl ResponseType for Disconnect {
    type Item = ();
    type Error = ();
}

impl Bus {
    fn remove_client_from_round_robin_state(&mut self, message: Disconnect) {
        debug!("removing client from round robin state: client='{}'", message.addr);
        if let Some(details) = self.sessions.get(&message.addr) {
            // We also need to remove any sticky consistency mappings for this client.
            for sticky_key in details.consistency_keys.iter() {
                debug!("attempting to remove consistency key: key='{:?}'", sticky_key);
                match self.sticky_consistency.remove(sticky_key) {
                    Some(_) => debug!("removed consistency key and client from sticky mapping: \
                                       key='{:?}' client='{}'", sticky_key, message.addr),
                    None => error!("consistency key was not in sticky consistency map: \
                                    key='{:?}'", sticky_key),
                }
            }

            // Now that we've found the client in our session map, find out which client type it
            // was.
            if let Some(ref client_type) = details.client_type {
                // Now we've got the client type, find the queue of clients of that type and
                // remove it from the queue.
                if let Some(queue) = self.round_robin_state.get_mut(client_type) {
                    queue.remove_item(&message.addr);
                    debug!("removed client from round robin state: \
                           client='{}' client_type='{}'",
                           message.addr, client_type);
                } else {
                    warn!("client was not in expected queue: client='{}'", message.addr);
                }
            } else {
                debug!("client did not have a client type: client='{}'", message.addr);
            }
        } else {
            error!("client is not present in sessions. this is a bug.");
        }
    }

    fn handle_unacknowledged_events(&mut self, message: Disconnect) -> Result<(), Error> {
        debug!("processing unacknowledged events for disconnecting client: client='{}'",
               message.addr);
        let (client_type, unacknowledged_events) = match self.sessions.get(&message.addr) {
            Some(details) => {
                let client_type = details.client_type.clone().ok_or(
                    Error::from(ErrorKind::UnacknowledgedEventResendWithoutClientType))?;
                (client_type, details.unacknowledged_events.clone())
            },
            None => return Err(Error::from(ErrorKind::SessionNotInHashMap)),
        };


        for unacknowledged_event in unacknowledged_events.iter() {
            debug!("re-propagating unacknowledged event: event=\n{}",
                   to_string_pretty(&unacknowledged_event)?);
            // We should convert this back to a event for sending - in the list it is meant for
            // matching with the expected incoming acks.
            let mut unacknowledged_event = unacknowledged_event.clone();
            unacknowledged_event.message_type = Some(String::from("event"));
            self.propagate_event_to_client_type(&unacknowledged_event, client_type.clone());
        }

        Ok(())
    }
}

impl Handler<Disconnect> for Bus {
    type Result = ();

    fn handle(&mut self, message: Disconnect, _: &mut Context<Self>) {
        info!("removing session from bus: client='{}'", message.addr);

        // Remove the client address from the round robin state.
        self.remove_client_from_round_robin_state(message.clone());

        // Process any unacknowledged events.
        if let Err(e) = self.handle_unacknowledged_events(message.clone()) {
            error!("handling unacknowledged events: error='{}'", e);
        }

        if let Some(_) = self.sessions.remove(&message.addr) {
            info!("removed session from bus: client='{}'", message.addr);
        } else {
            warn!("failed to remove session from bus: client='{}'", message.addr);
        }
    }
}
