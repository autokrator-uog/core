use std::clone::Clone;
use std::net::SocketAddr;

use actix::{Context, Handler, ResponseType};
use failure::Error;
use serde::Serialize;

use bus::{Bus, SessionDetails, RegisteredTypes};
use error::ErrorKind;
use signals::SendToClient;

/// The `SendToAllClients` message is sent to the Bus when a message needs to be sent
/// to all the clients managed by that session.
pub struct SendToAllClients<T: Serialize + Send + Clone>(pub T, pub String);

impl<T: Send> ResponseType for SendToAllClients<T>
    where T: Serialize + Clone
{
    type Item = ();
    type Error = ();
}

impl Bus {
    fn should_send_to_client(&self, socket: SocketAddr, details: SessionDetails,
                                 event_type: String) -> bool {
        let should_send = match details.registered_types {
            RegisteredTypes::All => true,
            RegisteredTypes::Some(ref types) => {
                if types.contains(&event_type) {
                    true
                } else {
                    false
                }
            },
        } && details.is_registered;

        info!("sending message registration check: client='{}' \
              registered_types='RegisteredTypes::{:?}' \
              type=`{}` is_registered='{:?}' sending='{:?}'",
              socket, details.registered_types, event_type, details.is_registered,
              should_send);
        should_send
    }

    fn next_round_robin_for_client_type(
            &mut self, client_type: &String) -> Result<(SocketAddr, SessionDetails), Error> {
        let socket = match self.round_robin_state.get_mut(client_type) {
            Some(queue) => {
                match queue.pop_front() {
                    Some(socket) => {
                        queue.push_back(socket.clone());
                        socket
                    },
                    None => return Err(Error::from(ErrorKind::RoundRobinEmptyQueue)),
                }
            },
            None => return Err(Error::from(ErrorKind::RoundRobinNoQueue)),
        };

        if let Some(details) = self.sessions.get(&socket) {
            Ok((socket.clone(), details.clone()))
        } else {
            Err(Error::from(ErrorKind::SessionNotInHashMap))
        }
    }

    pub fn send_to_all_clients<T: Serialize + Send + Clone + 'static>(&mut self, event: T,
                                                                      event_type: String) {
        let types = self.round_robin_state.keys().cloned().collect::<Vec<_>>();
        for client_type in types {
            // For each client type, we take the next available round robin selected client.
            if let Ok((socket, details)) = self.next_round_robin_for_client_type(&client_type) {
                info!("round robin selection: client='{}'", socket);

                // We need to check whether we should send to this client. In theory, this could
                // cause a certain client type to miss an event entirely if this were to return
                // false. However, given that all instances of a client type should be consistent
                // in which event types they are interested in, in practice this shouldn't cause
                // an issue.
                if self.should_send_to_client(socket, details.clone(), event_type.clone()) {
                    info!("sending 'send to all' signal: client='{}'", socket);
                    let cloned = event.clone();
                    details.address.send(SendToClient(cloned));
                } else {
                    info!("not sending 'send to all' signal: client='{}'", socket);
                }

            } else {
                warn!("round robin selection failed");
            }
        }
    }
}

impl<T> Handler<SendToAllClients<T>> for Bus
    where T: Serialize + Send + Clone + 'static
{
    type Result = ();

    fn handle(&mut self, message: SendToAllClients<T>, _: &mut Context<Self>) {
        self.send_to_all_clients(message.0, message.1);
    }
}
