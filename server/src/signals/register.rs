use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;

use actix::{Address, Context, Handler, ResponseType};
use common::VecDequeExt;
use common::schemas::{Register as RegisterSchema, Registration};
use failure::{Error, ResultExt};
use serde_json::{from_str, to_string_pretty};

use bus::{Bus, RegisteredTypes};
use error::ErrorKind;
use session::Session;
use signals::SendToClient;

/// The `Register` message is sent to the Bus when a client wants to provide more information about
/// itself or limit event types it can receive.
pub struct Register {
    pub message: String,
    pub bus: Address<Bus>,
    pub sender: (Address<Session>, SocketAddr),
}

impl ResponseType for Register {
    type Item = ();
    type Error = ();
}

impl Bus {
    fn update_round_robin_state_from_registration(&mut self, socket: SocketAddr,
                                                  parsed: RegisterSchema) -> Result<(), Error> {
        info!("updating client type: client='{}' type='{}'", socket, parsed.client_type);
        match self.sessions.get_mut(&socket) {
            Some(details) => {
                // We need to remove the socket from any existing lists before adding it to any
                // new ones.
                match details.client_type {
                    Some(ref existing_type) => {
                        match self.round_robin_state.get_mut(existing_type) {
                            Some(queue) => {
                                queue.remove_item(&socket);
                                ()
                            },
                            None => error!("existing client type not in round robin state: \
                                           type='{}'",
                                           existing_type),
                        }
                    },
                    None => debug!("client did not have client type previously: \
                                   client='{}'", socket),
                }

                // Update the details struct.
                details.client_type = Some(parsed.client_type.clone());
            },
            None => {
                error!("client is not present in sessions. this is a bug.");
                return Err(Error::from(ErrorKind::SessionNotInHashMap));
            },
        }

        // Update the list for this type.
        match self.round_robin_state.entry(parsed.client_type.clone()) {
            Entry::Occupied(mut entry) => {
                debug!("adding client to existing client type queue: \
                      client='{}' type='{}'",
                      socket, parsed.client_type);
                let mut queue = entry.get_mut();
                queue.push_back(socket.clone())
            },
            Entry::Vacant(entry) => {
                debug!("adding client to new client type queue: \
                      client='{}' type='{}'",
                      socket, parsed.client_type);
                let mut queue = VecDeque::new();
                queue.push_back(socket.clone());
                entry.insert(queue);
            },
        }

        Ok(())
    }

    fn update_sessions_from_registration(&mut self, socket: SocketAddr,
                                         parsed: RegisterSchema) -> Result<(), Error> {
        match self.sessions.get_mut(&socket) {
            Some(details) => {
                if parsed.event_types.len() == 1 && parsed.event_types[0] == "*" {
                    info!("updated register types for client: client='{}' types=all", socket);
                    details.registered_types = RegisteredTypes::All;
                } else {
                    info!("updated register types for client: client='{}' types='{:?}'",
                          socket, parsed.event_types);
                    details.registered_types = RegisteredTypes::Some(parsed.event_types.clone());
                }

                Ok(())
            },
            None => {
                error!("client is not present in sessions. this is a bug.");
                Err(Error::from(ErrorKind::SessionNotInHashMap))
            },
        }
    }

    pub fn resend_events_for_client_type(&mut self, client_type: String) -> Result<(), Error> {
        debug!("checking for events pending propagation for this client type");
        let pending_events = match self.pending_events.entry(client_type.clone()) {
            Entry::Occupied(mut entry) => {
                debug!("client type in pending events - replacing with empty vector of events");
                let events = entry.insert(Vec::new());
                events
            },
            Entry::Vacant(entry) => {
                debug!("client type not in pending events - found no pending events to send, \
                       adding empty vec: client_type='{}'",
                       client_type);
                entry.insert(Vec::new());
                return Ok(());
            },
        };

        debug!("client type in pending events - resending any pending events");
        for event in pending_events {
            self.propagate_event_to_client_type(&event, client_type.clone());
        }
        Ok(())
    }

    pub fn register(&mut self, message: Register) -> Result<(), Error> {
        let (addr, socket) = message.sender;

        let parsed: RegisterSchema = from_str(&message.message).context(
            ErrorKind::ParseNewEventMessage)?;
        info!("parsed register message: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);

        self.update_sessions_from_registration(socket, parsed.clone())?;
        self.update_round_robin_state_from_registration(socket, parsed.clone())?;

        // We can resend events for this client type now that a client is connected and registered,
        // if an event is in our global resend list then that means there were no clients
        // connected when the service initially went down and so this is the first client of that
        // type to come back.
        self.resend_events_for_client_type(parsed.client_type.clone())?;

        let response = Registration {
            client_type: parsed.client_type.clone(),
            event_types: parsed.event_types.clone(),
            message_type: "registration".to_string(),
        };

        info!("sending receipt to the client");
        addr.send(SendToClient(response));
        Ok(())
    }
}


impl Handler<Register> for Bus {
    type Result = ();

    fn handle(&mut self, message: Register, _: &mut Context<Self>) {
        if let Err(e) = self.register(message) {
            error!("processing new event: error='{}'", e);
        }
    }
}
