use actix::{Context, Handler, ResponseType};
use common::schemas::{NewEvent, Receipts};
use failure::{Error, ResultExt};
use rlua::Function;
use serde_json::{from_str, to_string_pretty};

use error::ErrorKind;
use interpreter::{Bus, Interpreter, json_to_lua};

/// The `Receipt` signal is sent from the client to the interpreter when a new receipt is received
/// from the event bus.
pub struct Receipt {
    pub message: String,
}

impl ResponseType for Receipt {
    type Item = ();
    type Error = ();
}

impl Interpreter {
    fn handle_receipt(&mut self, receipt: Receipt) -> Result<(), Error> {
        let parsed: Receipts = from_str(&receipt.message).context(ErrorKind::ParseReceiptMessage)?;
        debug!("received receipt: message=\n{}", to_string_pretty(&parsed)?);

        let bus: Bus = {
            let globals = self.lua.globals();
            globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?
        };

        for receipt in parsed.receipts {
            let event: NewEvent = match self.receipt_lookup.get(&receipt.checksum) {
                Some(event) => event.clone(),
                None => {
                    error!("receipt not found in hashmap: hash='{}'", receipt.checksum);
                    continue;
                },
            };
            debug!("matched event in receipt: message=\n{}", to_string_pretty(&event)?);

            debug!("checking consistency updates from receipt");
            self.increment_consistency_if_required(event.consistency.key.clone(),
                                                   event.consistency.value)?;

            match bus.receipt_handlers.get(&event.event_type.clone()) {
                Some(key) => {
                    let function: Function = self.lua.named_registry_value(key).context(
                        ErrorKind::MissingReceiptHandlerRegistryValue)?;

                    debug!("calling receipt handler");
                    let data = json_to_lua(&self.lua, event.data).context(
                        ErrorKind::ParseReceiptMessage)?;
                    let args = (receipt.status, event.event_type, event.consistency.key,
                                event.correlation_id, data);
                    if let Err(e) = function.call::<_, ()>(args) {
                        error!("failure running receipt handler: \n\n{}\n", e);
                    }
                    debug!("finished receipt handler");
                },
                None => {
                    warn!("missing receipt handler for event type: error='{}'",
                          Error::from(ErrorKind::MissingReceiptHandlerRegistryValue));
                }
            }
        }

        Ok(())
    }
}

impl Handler<Receipt> for Interpreter {
    type Result = ();

    fn handle(&mut self, receipt: Receipt, _: &mut Context<Self>) {
        info!("received receipt signal from client");
        if let Err(e) = self.handle_receipt(receipt) {
            match &e.downcast::<ErrorKind>() {
                &Ok(ErrorKind::MissingReceiptHandlerRegistryValue) => {
                    warn!("no handler for receipt");
                },
                // Not able to collapse these two conditions into a single condition.
                &Ok(ref e) => error!("processing receipt: error='{}'", e),
                &Err(ref e) => error!("processing receipt: error='{}'", e),
            }
        }
    }
}
