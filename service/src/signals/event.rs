use actix::{Context, Handler, ResponseType};
use failure::{Error, ResultExt};
use rlua::Function;
use serde_json::{Value, from_str};

use error::ErrorKind;
use interpreter::{Bus, Interpreter};

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
        let parsed: Value = from_str(&event.message)?;
        let event_type: String = String::from(parsed["event_type"].as_str().unwrap());

        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;

        match bus.event_handlers.get(&event_type) {
            Some(key) => {
                let function: Function = self.lua.named_registry_value(key).context(
                    ErrorKind::MissingEventHandlerRegistryValue)?;

                debug!("calling event handler");
                function.call::<_, ()>(event.message).context(
                    ErrorKind::FailedEventHandler).map_err(Error::from)
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
