use actix::{Context, Handler, ResponseType};
use common::schemas::{NewEvent, Receipts};
use failure::{Error, ResultExt};
use rlua::Function;
use serde_json::from_str;

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

        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;
        for receipt in parsed.receipts {
            let event: NewEvent = match self.receipt_lookup.get(&receipt.checksum) {
                Some(event) => event.clone(),
                None => {
                    error!("receipt not found in hashmap: hash='{}'", receipt.checksum);
                    continue;
                },
            };

            match bus.receipt_handlers.get(&event.event_type.clone()) {
                Some(key) => {
                    let function: Function = self.lua.named_registry_value(key).context(
                        ErrorKind::MissingReceiptHandlerRegistryValue)?;

                    debug!("calling receipt handler");
                    let data = json_to_lua(&self.lua, event.data).context(
                        ErrorKind::ParseReceiptMessage)?;
                    let args = (event.event_type, event.consistency.key, event.correlation_id,
                                data);
                    if let Err(e) = function.call::<_, ()>(args).context(
                            ErrorKind::FailedReceiptHandler).map_err(Error::from) {
                        error!("running receipt handler: error='{}'", e);
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
            error!("processing receipt: error='{}'", e);
        }
    }
}
