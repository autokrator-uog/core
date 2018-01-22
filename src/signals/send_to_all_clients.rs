use std::clone::Clone;

use actix::{Actor, Context, Handler, Response, ResponseType};
use serde::Serialize;

use bus::{Bus, RegisteredTypes};
use signals::SendToClient;

/// The `SendToAllClients` message is sent to the Bus when a message needs to be sent
/// to all the clients managed by that session.
pub struct SendToAllClients<T: Serialize + Clone>(pub T, pub String);

impl<T> ResponseType for SendToAllClients<T>
    where T: Serialize + Clone
{
    type Item = ();
    type Error = ();
}

impl Bus {
    pub fn send_to_all_clients<T: Serialize + Clone + 'static>(&mut self, event: T,
                                                               event_type: String) {
        for (socket, details) in &self.sessions {
            let should_send = match details.registered_types {
                RegisteredTypes::All => true,
                RegisteredTypes::Some(ref types) => {
                    if types.contains(&event_type) {
                        true
                    } else {
                        false
                    }
                },
            };

            info!("sending message registration check: client=`{:?}` \
                  registered_types=`RegisteredTypes::{:?}` \
                  type=`{}` sending=`{:?}`",
                  socket, details.registered_types, event_type, should_send);
            if should_send {
                let cloned = event.clone();
                details.address.send(SendToClient(cloned));
            }
        }
    }
}

impl<T> Handler<SendToAllClients<T>> for Bus
    where T: Serialize + Clone + 'static
{
    fn handle(&mut self, message: SendToAllClients<T>,
              _: &mut Context<Self>) -> Response<Self, SendToAllClients<T>> {
        self.send_to_all_clients(message.0, message.1);
        Self::empty()
    }
}
