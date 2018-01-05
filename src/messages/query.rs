use actix::{Actor, Address, Context, Handler, Response, ResponseType};

use bus::Bus;
use session::Session;

/// The `Query` message is sent to the Bus when query requests are sent from websockets.
pub struct Query {
    pub message: String,
    pub sender: Address<Session>,
    pub bus: Address<Bus>,
}

impl ResponseType for Query {
    type Item = ();
    type Error = ();
}

impl Handler<Query> for Bus {
    fn handle(&mut self, _message: Query, _: &mut Context<Self>) -> Response<Self, Query> {
        Self::empty()
    }
}
