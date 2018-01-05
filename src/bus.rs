use std::collections::HashMap;
use std::net::SocketAddr;

use actix::{Actor, Address, Context};

use session::Session;

/// Bus maintains the state that pertains to all clients and allows clients to send messages
/// to each other.
///
/// Handlers for different types of messages that the bus can handle are implemented in the
/// messages module.
pub struct Bus {
    pub sessions: HashMap<SocketAddr, Address<Session>>,
}

impl Default for Bus {
    fn default() -> Self {
        Bus {
            sessions: HashMap::new(),
        }
    }
}

impl Actor for Bus {
    type Context = Context<Self>;
}

