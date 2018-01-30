use actix::{Context, Handler, ResponseType};
use rlua::Function;

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

impl Handler<Event> for Interpreter {
    type Result = ();

    fn handle(&mut self, message: Event, _: &mut Context<Self>) {
        info!("received event signal from client");
        let func: Result<Function, _> = self.lua.named_registry_value(
            LUA_EVENT_HANDLER_REGISTRY_KEY);
        match func {
            Ok(func) => {
                debug!("calling event handler");
                if let Err(e) = func.call::<_, ()>(message.message) {
                    error!("event handler: error='{:?}'", e);
                }
            },
            Err(e) => error!("failed to find event handler in lua registry: error='{:?}'", e),
        }
    }
}
