use std::clone::Clone;
use std::net::SocketAddr;

use actix::{Context, Handler, ResponseType};
use failure::{Error, ResultExt};
use serde::Serialize;
use serde_json::to_string;
use vicarius_common::schemas::outgoing::Consistency;

use bus::{Bus, SessionDetails, RegisteredTypes};
use error::ErrorKind;
use signals::SendToClient;

/// The `PropagateEvent` message is sent to the Bus when a message needs to be sent
/// to all appropriate clients. This should not be used for sending receipts, registrations or
/// any one-off message to clients - it is intended for use of the sticky round robin system for
/// distributing events.
pub struct PropagateEvent<T: Consistency + Serialize + Send + Clone>(pub T, pub String);

impl<T: Send> ResponseType for PropagateEvent<T>
    where T: Consistency + Serialize + Clone
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
        } && details.client_type.is_some();

        info!("sending message registration check: client='{}' \
              registered_types='RegisteredTypes::{:?}' \
              type='{}' is_registered='{:?}' sending='{:?}'",
              socket, details.registered_types, event_type, details.client_type.is_some(),
              should_send);
        should_send
    }

    fn next_client_for_sending<T>(&mut self, event: T,
                                  client_type: &String) -> Result<(SocketAddr, SessionDetails), Error>
        where T: Consistency + Serialize + Send + Clone + 'static
    {
        let sticky_key = (client_type.clone(), event.consistency_key());
        let socket = if let Some(socket) = self.sticky_consistency.get(&sticky_key) {
            debug!("found sticky client for: key='{}'", event.consistency_key());
            *socket
        } else {
            debug!("finding non-sticky client for: client_type='{}'", client_type);
            match self.round_robin_state.get_mut(client_type) {
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
            }
        };

        if let Some(details) = self.sessions.get_mut(&socket) {
            // Ensure that this client always receives this consistency key in future.
            self.sticky_consistency.insert(sticky_key.clone(), socket);
            details.consistency_keys.insert(sticky_key);

            // Keep track of this event as unawknowledged.
            let serialized_event = to_string(&event).context(ErrorKind::ParseJsonFromKafka)?;
            details.unawknowledged_events.insert(serialized_event);

            Ok((socket.clone(), details.clone()))
        } else {
            Err(Error::from(ErrorKind::SessionNotInHashMap))
        }
    }

    pub fn propagate_event<T>(&mut self, event: T, event_type: String)
        where T: Consistency + Serialize + Send + Clone + 'static
    {
        let types = self.round_robin_state.keys().cloned().collect::<Vec<_>>();
        debug!("checking client types: client_types='{:?}'", types);
        for client_type in types {
            info!("sending to client type: client_type='{}'", client_type);
            // For each client type, we take the next available round robin selected client.
            if let Ok((socket, details)) = self.next_client_for_sending(event.clone(),
                                                                        &client_type) {
                info!("client selection: client='{}'", socket);

                // We need to check whether we should send to this client. In theory, this could
                // cause a certain client type to miss an event entirely if this were to return
                // false. However, given that all instances of a client type should be consistent
                // in which event types they are interested in, in practice this shouldn't cause
                // an issue.
                if self.should_send_to_client(socket, details.clone(), event_type.clone()) {
                    info!("sending 'send to client' signal: client='{}'", socket);
                    details.address.send(SendToClient(event.clone()));
                } else {
                    info!("not sending 'send to client' signal: client='{}'", socket);
                }

            } else {
                warn!("round robin selection failed");
            }
        }
    }
}

impl<T> Handler<PropagateEvent<T>> for Bus
    where T: Consistency + Serialize + Send + Clone + 'static
{
    type Result = ();

    fn handle(&mut self, message: PropagateEvent<T>, _: &mut Context<Self>) {
        debug!("received propagate event signal");
        self.propagate_event(message.0, message.1);
    }
}
