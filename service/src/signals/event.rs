use std::collections::hash_map::Entry;

use actix::{Context, Handler, ResponseType};
use common::schemas::{
    ConsistencyKey,
    ConsistencyValue,
    Event as EventSchema,
};
use failure::{Error, Fail, ResultExt};
use redis::Commands;
use rlua::Function;
use serde_json::{from_str, to_string_pretty};

use error::ErrorKind;
use interpreter::{TIMESTAMP_KEY, Bus, Interpreter, json_to_lua};
use signals::SendMessage;

/// The `Event` signal is sent from the client to the interpreter when a new event is received from
/// the event bus.
pub struct Event {
    pub message: String,
}

impl ResponseType for Event {
    type Item = ();
    type Error = ();
}

impl Interpreter {
    pub fn increment_consistency_if_required(&mut self, key: ConsistencyKey,
                                             value: ConsistencyValue) -> Result<(), Error> {
        // Increment the consistency value if the consistency value in this event is higher than
        // what we have locally.
        debug!("checking for consistency increment");
        match self.consistency.entry(key) {
            Entry::Occupied(mut entry) => {
                let is_higher = {
                    let current = entry.get();
                    debug!("current consistency value for key: current='{}' \
                           received_value='{}'", current, value);
                    value > *current
                };
                debug!("higher consistency value: is_higher='{}'", is_higher);

                if is_higher {
                    debug!("replacing value with new consistency value: value='{}'", value);
                    entry.insert(value);
                }
            },
            Entry::Vacant(entry) => {
                debug!("new consistency key, inserting new value: value='{}'", value);
                entry.insert(value.clone());
                ()
            },
        }
        Ok(())
    }

    pub fn save_timestamp_for_query(&mut self, event: &EventSchema) -> Result<(), Error> {
        let key = String::from(TIMESTAMP_KEY);
        let value = event.timestamp_raw.ok_or(Error::from(ErrorKind::NoTimestampProvided))?;
        debug!("persisting timestamp: timestamp_raw='{}'", value);
        self.redis.set::<String, i64, _>(key, value).context(
            ErrorKind::RedisPersist).map_err(Error::from)
    }

    fn handle_event(&mut self, event: Event) -> Result<(), Error> {
        let parsed: EventSchema = from_str(&event.message).context(ErrorKind::ParseEventMessage)?;
        debug!("received event: message=\n{}", to_string_pretty(&parsed)?);
        // We'll send this if handler succeeds.
        let mut awknowledgement = parsed.clone();

        debug!("saving timestamp for query");
        self.save_timestamp_for_query(&parsed)?;

        debug!("checking consistency updates from event");
        self.increment_consistency_if_required(parsed.consistency.key.clone(),
                                               parsed.consistency.value)?;

        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;
        match bus.event_handlers.get(&parsed.event_type.clone()) {
            Some(key) => {
                let function: Function = self.lua.named_registry_value(key).context(
                    ErrorKind::MissingEventHandlerRegistryValue)?;

                debug!("calling event handler");
                let data = json_to_lua(&self.lua, parsed.data).context(
                    ErrorKind::ParseEventMessage)?;
                let args = (parsed.event_type, parsed.consistency.key, parsed.correlation_id,
                            data);
                if let Err(e) = function.call::<_, ()>(args) {
                    error!("failure running event hander: \n\n{}\n", e);
                    return Err(Error::from(e.context(ErrorKind::FailedEventHandler)));
                }

                // Respond with an awknowledgement.
                awknowledgement.message_type = Some(String::from("ack"));
                if let Some(ref client) = self.client {
                    info!("responding with awknowledgement");
                    client.send(SendMessage(awknowledgement));
                } else {
                    return Err(Error::from(ErrorKind::ClientNotLinkedToInterpreter));
                }

                Ok(())
            },
            None => return Err(Error::from(ErrorKind::MissingEventHandlerRegistryValue)),
        }
    }
}

impl Handler<Event> for Interpreter {
    type Result = ();

    fn handle(&mut self, event: Event, _: &mut Context<Self>) {
        info!("received event signal from client");
        if let Err(e) = self.handle_event(event) {
            error!("processing event: error='{}'", e);
        }
    }
}
