use actix::{Context, Handler, ResponseType};
use common::schemas::Rebuild as RebuildSchema;
use failure::{Error, Fail, ResultExt};
use rlua::Function;
use serde_json::{from_str, to_string_pretty};

use error::ErrorKind;
use interpreter::{Bus, Interpreter, json_to_lua};

/// The `Rebuild` signal is sent from the client to the interpreter when a new event from a query
/// is received from the event bus.
pub struct Rebuild {
    pub message: String,
}

impl ResponseType for Rebuild {
    type Item = ();
    type Error = ();
}

impl Interpreter {
    fn handle_rebuild(&mut self, event: Rebuild) -> Result<(), Error> {
        let parsed: RebuildSchema = from_str(&event.message).context(
            ErrorKind::ParseEventMessage)?;
        debug!("received rebuild event: message=\n{}", to_string_pretty(&parsed)?);

        for event in parsed.events {
            debug!("saving timestamp for query");
            self.save_timestamp_for_query(&event)?;

            debug!("checking consistency updates from event");
            self.increment_consistency_if_required(event.consistency.key.clone(),
                                                   event.consistency.value)?;

            let globals = self.lua.globals();
            let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;
            match bus.rebuild_handlers.get(&event.event_type.clone()) {
                Some(key) => {
                    let function: Function = self.lua.named_registry_value(key).context(
                        ErrorKind::MissingRebuildHandlerRegistryValue)?;

                    debug!("calling event handler");
                    let data = json_to_lua(&self.lua, event.data).context(
                        ErrorKind::ParseEventMessage)?;
                    let args = (event.event_type, event.consistency.key, event.correlation_id,
                                data);
                    if let Err(e) = function.call::<_, ()>(args) {
                        error!("failure running rebuild hander: \n\n{}\n", e);
                        return Err(Error::from(e.context(ErrorKind::FailedRebuildHandler)));
                    }
                },
                None => return Err(Error::from(ErrorKind::MissingRebuildHandlerRegistryValue)),
            }
        }

        Ok(())
    }
}

impl Handler<Rebuild> for Interpreter {
    type Result = ();

    fn handle(&mut self, event: Rebuild, _: &mut Context<Self>) {
        info!("received rebuild signal from client");
        if let Err(e) = self.handle_rebuild(event) {
            error!("processing rebuild: error='{}'", e);
        }
    }
}
