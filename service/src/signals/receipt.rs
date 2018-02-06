use actix::{Context, Handler, ResponseType};
use failure::Error;

use interpreter::Interpreter;

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
    fn handle_receipt(&mut self, _receipt: Receipt) -> Result<(), Error> {
        error!("not yet implemented");
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
