use std::clone::Clone;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;

use actix::{Context, Handler, ResponseType};
use common::schemas::Event;
use failure::Error;

use bus::{Bus, SessionDetails, RegisteredTypes};
use error::ErrorKind;
use signals::SendToClient;

/// The `PropagateEvent` message is sent to the Bus when a message needs to be sent
/// to all appropriate clients. This should not be used for sending receipts, registrations or
/// any one-off message to clients - it is intended for use of the sticky round robin system for
/// distributing events.
pub struct PropagateEvent {
    pub event: Event,
}

impl ResponseType for PropagateEvent {
    type Item = ();
    type Error = ();
}

enum ShouldSend {
    Yes(SocketAddr, SessionDetails),
    No,
}

fn should_send_to_client_type(registered_types: &RegisteredTypes, event_type: &str,
                              client_type: &Option<String>) -> bool {
    let is_registered = match *registered_types {
        RegisteredTypes::All => true,
        RegisteredTypes::Some(ref types) => {
            if types.contains(&event_type.to_owned()) {
                true
            } else {
                false
            }
        },
    };
    let has_client_type = client_type.is_some();
    is_registered && has_client_type
}

impl Bus {
    fn next_client_for_sending(&mut self, event: Event,
                               client_type: &String) -> Result<ShouldSend, Error>
    {
        let sticky_key = (client_type.clone(), event.consistency.key.clone());
        let socket = if let Some(socket) = self.sticky_consistency.get(&sticky_key) {
            debug!("found sticky client for: key='{}'", event.consistency.key);
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

            // We need to check whether we should send to this client. In theory, this could
            // cause a certain client type to miss an event entirely if this were to return
            // false. However, given that all instances of a client type should be consistent
            // in which event types they are interested in, in practice this shouldn't cause
            // an issue.
            if should_send_to_client_type(&details.registered_types, &event.event_type,
                                          &details.client_type) {
                info!("sending 'send to client' signal: client='{}'", socket);
                // Keep track of this event as unacknowledged.
                let mut expected_ack_event = event.clone();
                expected_ack_event.message_type = Some(String::from("ack"));
                details.unacknowledged_events.insert(expected_ack_event);

                Ok(ShouldSend::Yes(socket.clone(), details.clone()))
            } else {
                info!("not sending 'send to client' signal: client='{}'", socket);
                Ok(ShouldSend::No)
            }
        } else {
            Err(Error::from(ErrorKind::SessionNotInHashMap))
        }
    }

    pub fn propagate_event_to_client_type(&mut self, event: &Event, client_type: String) {
        // For each client type, we take the next available round robin selected client.
        match self.next_client_for_sending(event.clone(), &client_type) {
            Ok(ShouldSend::Yes(socket, details)) => {
                info!("client selection: client='{}'", socket);
                details.address.send(SendToClient(event.clone()));
            },
            // If we aren't registered for this event, this client type will never be registered,
            // don't mark as pending.
            Ok(ShouldSend::No) => {},
            Err(_) => {
                warn!("round robin selection failed, saving for resend at later time");
                match self.pending_events.entry(client_type.clone()) {
                    Entry::Occupied(mut entry) => {
                        debug!("adding another pending event for client type: client_type='{}'",
                               client_type);
                        let mut existing_events = { entry.get().clone() };
                        existing_events.push(event.clone());
                        entry.insert(existing_events);
                    },
                    Entry::Vacant(entry) => {
                        debug!("adding first pending event for client type: client_type='{}'",
                               client_type);
                        entry.insert(vec![event.clone()]);
                    },
                }
            },
        };
    }

    pub fn propagate_event(&mut self, event: Event) {
        let types = self.round_robin_state.keys().cloned().collect::<Vec<_>>();
        debug!("checking client types: client_types='{:?}'", types);
        for client_type in types {
            info!("sending to client type: client_type='{}'", client_type);
            self.propagate_event_to_client_type(&event, client_type);
        }
    }
}

impl Handler<PropagateEvent> for Bus {
    type Result = ();

    fn handle(&mut self, message: PropagateEvent, _: &mut Context<Self>) {
        debug!("received propagate event signal");
        self.propagate_event(message.event);
    }
}
