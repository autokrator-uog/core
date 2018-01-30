use actix::{Context, Handler, ResponseType};

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

impl Handler<Receipt> for Interpreter {
    type Result = ();

    fn handle(&mut self, _message: Receipt, _: &mut Context<Self>) {
        info!("received receipt signal from client");
    }
}
