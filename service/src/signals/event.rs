use actix::{Context, Handler, ResponseType};
use common::schemas::Event as EventSchema;
use failure::{Error, Fail, ResultExt};
use rlua::Function;
use serde_json::from_str;

use error::ErrorKind;
use interpreter::{Bus, Interpreter, json_to_lua};

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
    fn handle_event(&mut self, event: Event) -> Result<(), Error> {
        let parsed: EventSchema = from_str(&event.message).context(ErrorKind::ParseEventMessage)?;

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
                } else {
                    Ok(())
                }
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
