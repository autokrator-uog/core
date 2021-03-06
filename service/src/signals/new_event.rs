use std::collections::hash_map::Entry;

use actix::{Context, Handler, ResponseType};
use common::hash_json;
use common::schemas::{
    Consistency,
    ConsistencyKey,
    ConsistencyValue,
    NewEvent as NewEventSchema,
    NewEvents,
};
use failure::Error;
use rand::Rng;
use serde_json::Value;

use error::ErrorKind;
use interpreter::Interpreter;
use signals::SendMessage;

/// The `NewEvent` signal is sent from the interpreter to the client when a new event needs to
/// be sent to the event bus.
pub struct NewEvent {
    pub consistency_key: ConsistencyKey,
    pub data: Value,
    pub event_type: String,
    pub implicit: bool,
    pub correlation_id: Option<u32>,
}

impl ResponseType for NewEvent {
    type Item = ();
    type Error = ();
}

impl Interpreter {
    pub fn send_new_event(&mut self, consistency_key: ConsistencyKey, data: Value,
                          event_type: String, implicit: bool,
                          correlation_id: Option<u32>) -> Result<(), Error> {
        let correlation_id = if let Some(correlation_id) = correlation_id {
            debug!("using existing correlation id: id='{}'", correlation_id);
            correlation_id
        } else {
            debug!("generating new correlation id");
            self.rng.borrow_mut().gen::<u32>()
        };

        debug!("checking consistency for send");
        let consistency_value = if implicit {
            debug!("implicit consistency");
            ConsistencyValue::Implicit
        } else {
            match self.consistency.entry(consistency_key.clone()) {
                Entry::Occupied(mut entry) => {
                    // We increment the consistency when we send events. As far as I can tell,
                    // there isn't a reason this will break.
                    //
                    // As long as we never send an event with a consistency key higher than
                    // expected, this should be fine.
                    let next_value = {
                        match entry.get() {
                            &ConsistencyValue::Explicit(v) => {
                                debug!("existing consistency key: existing_value='{}' \
                                       sending_value='{}'", v, v + 1);
                                v + 1
                            },
                            _ => return Err(Error::from(ErrorKind::ImplicitConsistencyInMap)),
                        }
                    };

                    let value = ConsistencyValue::Explicit(next_value);
                    debug!("using and updating consistency value: value='{}'", next_value);
                    entry.insert(value.clone());
                    value
                },
                Entry::Vacant(entry) => {
                    let initial = ConsistencyValue::Explicit(0);
                    debug!("new consistency key: value='{}'", initial);
                    entry.insert(initial.clone());
                    initial
                }
            }
        };

        let consistency = Consistency {
            key: consistency_key,
            value: consistency_value
        };

        let event = NewEventSchema {
            consistency: consistency,
            correlation_id: correlation_id,
            data: data.clone(),
            event_type: event_type,
        };

        let message = NewEvents {
            events: vec![ event.clone() ],
            message_type: "new".to_owned(),
        };

        if let Some(_) = self.receipt_lookup.insert(hash_json(&data)?, event) {
            info!("replaced event in receipt lookup - hash collision?");
        } else {
            info!("added event to receipt lookup");
        }

        if let Some(ref client) = self.client {
            debug!("sending send message signal");
            client.send(SendMessage(message));
            Ok(())
        } else {
            Err(Error::from(ErrorKind::ClientNotLinkedToInterpreter))
        }
    }
}

impl Handler<NewEvent> for Interpreter {
    type Result = ();

    fn handle(&mut self, new_event: NewEvent, _: &mut Context<Self>) {
        info!("received new event signal from interpreter");
        if let Err(e) = self.send_new_event(
            new_event.consistency_key,
            new_event.data,
            new_event.event_type,
            new_event.implicit,
            new_event.correlation_id,
        ) {
            error!("unable to send new event: error='{}'", e);
        }
    }
}
