use std::net::SocketAddr;

use actix::{Address, Context, Handler, ResponseType};

use bus::{Bus, SessionDetails, RegisteredTypes};
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
    type Result = ();

    fn handle(&mut self, message: Connect, _: &mut Context<Self>) {
        let details = SessionDetails {
            address: message.session,
            registered_types: RegisteredTypes::All,
            is_registered: false,
            client_type: None,
        };

        if let Some(_) = self.sessions.insert(message.addr, details) {
            info!("session updated in bus: client='{}'", message.addr);
        } else {
            info!("new session added to bus: client='{}'", message.addr);
        }
    }
}
