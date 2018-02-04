use std::process::exit;

use actix::{Context, Handler, SyncAddress, ResponseType};
use failure::{Error, ResultExt};
use serde_json::from_str;
use vicarius_common::schemas::incoming::{NewEventMessage, RegisterMessage};

use client::Client;
use error::ErrorKind;
use interpreter::{Interpreter, LUA_EVENT_TYPES_REGISTRY_KEY, LUA_CLIENT_TYPE_REGISTRY_KEY};
use signals::SendMessage;

/// The `Link` signal is sent from the client to the interpreter when the client starts so that
/// the register message can be sent.
pub struct Link {
    pub client: SyncAddress<Client>,
}

impl ResponseType for Link {
    type Item = ();
    type Error = ();
}

impl Interpreter {
    fn link_client(&mut self, client: SyncAddress<Client>) -> Result<(), Error> {
        // Set the client field on the interpreter.
        self.client = Some(client.clone());

        // The send function needs access to the client field of the interpreter and it
        // must be `Some(..)` when called.
        debug!("injecting send function");
        self.inject_send_function()?;

        // For our script to not error, we need to ensure everything is injected before running
        // the script.
        debug!("evaluating script");
        let script = self.script.clone();
        self.lua.eval::<()>(&script, None).context(ErrorKind::EvaluateLuaScript)?;

        // Now we have evaluated the script, register.
        self.send_register_message(client)
    }

    fn inject_send_function(&self) -> Result<(), Error> {
        // We can't clone the lua object, but we can clone the addresses that we intend to use
        // within the closure.
        let client = self.client.clone().ok_or(ErrorKind::ClientNotLinkedToInterpreter)?;
        let globals = self.lua.globals();

        let send_function = self.lua.create_function(move |_lua, message: String| {
            info!("received send call from script");

            let parsed: Result<NewEventMessage, _> = from_str(&message);
            if let Ok(message) = parsed {
                client.send(SendMessage(message));
            } else {
                error!("failed to parse new event message from lua: message=\n{}", message);
            }

            Ok(())
        }).context(ErrorKind::CreateSendFunction)?;
        globals.set("send", send_function).context(ErrorKind::InjectSendFunction)?;

        Ok(())
    }

    fn send_register_message(&mut self, client: SyncAddress<Client>) -> Result<(), Error> {
        let event_types: Vec<String> = self.lua.named_registry_value(
            LUA_EVENT_TYPES_REGISTRY_KEY).context(ErrorKind::MissingEventTypesRegistryValue)?;
        let client_type: String = self.lua.named_registry_value(
            LUA_CLIENT_TYPE_REGISTRY_KEY).context(ErrorKind::MissingClientTypeRegistryValue)?;

        info!("sending register message to server: event_types='{:?}' client_type='{}'",
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
        if let Err(e) = self.link_client(message.client.clone()) {
            error!("closing service, failure to link client: error='{:?}'", e);
            exit(1);
        }
    }
}
