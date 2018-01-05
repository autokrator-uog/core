use actix::{Actor, Address, Context, Handler, Response, ResponseType};

use bus::Bus;
use session::Session;

/// The `Register` message is sent to the Bus when a client wants to provide more information about
/// itself or limit event types it can receive.
pub struct Register {
    pub message: String,
    pub sender: Address<Session>,
    pub bus: Address<Bus>,
}

impl ResponseType for Register {
    type Item = ();
    type Error = ();
}

impl Handler<Register> for Bus {
    fn handle(&mut self, _message: Register, _: &mut Context<Self>) -> Response<Self, Register> {
        Self::empty()
    }
}
