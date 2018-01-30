use std::process::exit;

use actix::{Address, Context, Handler, ResponseType};
use failure::{Error, ResultExt};
use vicarius_common::schemas::incoming::RegisterMessage;

use client::Client;
use error::ErrorKind;
use interpreter::{Interpreter, LUA_EVENT_TYPES_REGISTRY_KEY, LUA_CLIENT_TYPE_REGISTRY_KEY};
use signals::SendMessage;

/// The `Link` signal is sent from the client to the interpreter when the client starts so that
/// the register message can be sent.
pub struct Link {
    pub client: Address<Client>,
}

impl ResponseType for Link {
    type Item = ();
    type Error = ();
}

impl Interpreter {
    fn send_register_message(&mut self, client: Address<Client>) -> Result<(), Error> {
        let event_types: Vec<String> = self.lua.named_registry_value(
            LUA_EVENT_TYPES_REGISTRY_KEY).context(ErrorKind::MissingEventTypesRegistryValue)?;
        let client_type: String = self.lua.named_registry_value(
            LUA_CLIENT_TYPE_REGISTRY_KEY).context(ErrorKind::MissingClientTypeRegistryValue)?;

        info!("sending register message to server: \
              event_types='{:?}' client_type='{}'",
              event_types, client_type);
        client.send(SendMessage(RegisterMessage {
            message_type: "register".to_owned(),
            event_types,
            client_type
        }));
        Ok(())
    }
}

impl Handler<Link> for Interpreter {
    type Result = ();

    fn handle(&mut self, message: Link, _: &mut Context<Self>) {
        info!("received link signal from client");
        if let Err(e) = self.send_register_message(message.client.clone()) {
            error!("closing service, register function was not invoked: error='{:?}'", e);
            exit(1);
        }
    }
}
