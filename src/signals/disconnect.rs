use std::net::SocketAddr;

use actix::{Context, Handler, ResponseType};

use bus::Bus;
use helpers::VecDequeExt;

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
}

impl Handler<Disconnect> for Bus {
    type Result = ();

    fn handle(&mut self, message: Disconnect,
              _: &mut Context<Self>) {
        info!("removing session from bus: client='{}'", message.addr);

        // Remove the client address from the round robin state.
        self.remove_client_from_round_robin_state(message.clone());

        if let Some(_) = self.sessions.remove(&message.addr) {
            info!("removed session from bus: client='{}'", message.addr);
        } else {
            warn!("failed to remove session from bus: client='{}'", message.addr);
        }
    }
}
