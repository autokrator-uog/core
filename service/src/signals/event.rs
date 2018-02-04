use actix::{Context, Handler, ResponseType};
use failure::{Error, ResultExt};
use rlua::Function;

use error::ErrorKind;
use interpreter::{LUA_EVENT_HANDLER_REGISTRY_KEY, Interpreter};

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
        let func: Function = self.lua.named_registry_value(LUA_EVENT_HANDLER_REGISTRY_KEY).context(
            ErrorKind::MissingEventHandlerRegistryValue)?;

        debug!("calling event handler");
        func.call::<_, ()>(event.message).context(
            ErrorKind::FailedEventHandler).map_err(Error::from)
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
