use std::net::SocketAddr;

use actix::{Actor, Context, Handler, Response, ResponseType};

use bus::Bus;

/// The `Disconnect` message is sent to the Bus when a client disconnects.
pub struct Disconnect {
    pub addr: SocketAddr,
}

impl ResponseType for Disconnect {
    type Item = ();
    type Error = ();
}

impl Handler<Disconnect> for Bus {
    fn handle(&mut self, message: Disconnect,
              _: &mut Context<Self>) -> Response<Self, Disconnect> {
        info!("removing session from broker: {}", message.addr);
        if let Some(_) = self.sessions.remove(&message.addr) {
            info!("removed session from broker: {}", message.addr);
        } else {
            warn!("failed to remove session from broker: {}", message.addr);
        }
        Self::empty()
    }
}
