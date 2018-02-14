use actix::{Context, Handler, ResponseType};

use interpreter::Interpreter;

/// The `Registration` signal is sent from the client to the interpreter when registration
/// confirmation is received from the event bus.
pub struct Registration {
    pub message: String,
}

impl ResponseType for Registration {
    type Item = ();
    type Error = ();
}

impl Handler<Registration> for Interpreter {
    type Result = ();

    fn handle(&mut self, _message: Registration, _: &mut Context<Self>) {
        info!("received registration signal from client");
    }
}
