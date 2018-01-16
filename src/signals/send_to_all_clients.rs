use std::clone::Clone;

use actix::{Actor, Context, Handler, Response, ResponseType};
use serde::Serialize;

use bus::Bus;
use signals::SendToClient;

/// The `SendToAllClients` message is sent to the Bus when a message needs to be sent
/// to all the clients managed by that session.
pub struct SendToAllClients<T: Serialize + Clone>(pub T);

impl<T> ResponseType for SendToAllClients<T>
    where T: Serialize + Clone
{
    type Item = ();
    type Error = ();
}

impl Bus {
    pub fn send_to_all_clients<T: Serialize + Clone + 'static>(&mut self, event: T) {
        for (_, addr) in &self.sessions {
            let cloned = event.clone();
            addr.send(SendToClient(cloned));
        }
    }
}

impl<T> Handler<SendToAllClients<T>> for Bus
    where T: Serialize + Clone + 'static
{
    fn handle(&mut self, message: SendToAllClients<T>,
              _: &mut Context<Self>) -> Response<Self, SendToAllClients<T>> {
        self.send_to_all_clients(message.0);
        Self::empty()
    }
}
