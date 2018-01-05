use std::net::SocketAddr;

use actix::{Actor, Address, Context, Handler, Response, ResponseType};

use bus::Bus;
use session::Session;

/// The `Connect` message is sent to the Bus when a client connects.
pub struct Connect {
    pub session: Address<Session>,
    pub addr: SocketAddr,
}

impl ResponseType for Connect {
    type Item = ();
    type Error = ();
}

impl Handler<Connect> for Bus {
    fn handle(&mut self, message: Connect, _: &mut Context<Self>) -> Response<Self, Connect> {
        if let Some(_) = self.sessions.insert(message.addr, message.session) {
            info!("session updated in broker: {}", message.addr);
        } else {
            info!("new session added to broker: {}", message.addr);
        }
        Self::empty()
    }
}
