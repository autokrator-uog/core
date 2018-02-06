use std::process::exit;

use actix::{AsyncContext, Context, Handler, SyncAddress, ResponseType};
use failure::{Error, ResultExt};
use vicarius_common::schemas::incoming::RegisterMessage;

use client::Client;
use error::ErrorKind;
use interpreter::{Bus, Interpreter};
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
    fn link_client(&mut self, client: SyncAddress<Client>,
                   ctx: &mut Context<Self>) -> Result<(), Error> {
        // Set the client field on the interpreter.
        self.client = Some(client.clone());

        let redis = self.redis.clone();
        {
            debug!("injecting bus userdata");
            let globals = self.lua.globals();
            globals.set("bus", Bus::new(ctx.address(), redis))?;
        }

        // For our script to not error, we need to ensure everything is injected before running
        // the script.
        debug!("evaluating script");
        let script = self.script.clone();
        if let Err(e) = self.lua.eval::<()>(&script, None) {
            error!("failed to evaluate script\n\n{}\n", e);
            return Err(Error::from(ErrorKind::EvaluateLuaScript));
        }
        debug!("finished script evaluation");

        // Now we have evaluated the script, register.
        self.send_register_message(client)
    }

    fn send_register_message(&mut self, client: SyncAddress<Client>) -> Result<(), Error> {
        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;

        let event_types = bus.event_types;
        let client_type = bus.client_type.ok_or(ErrorKind::ClientNotLinkedToInterpreter)?;

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

    fn handle(&mut self, message: Link, ctx: &mut Context<Self>) {
        info!("received link signal from client");
        if let Err(e) = self.link_client(message.client.clone(), ctx) {
            error!("closing service, failure to link client: error='{:?}'", e);
            exit(1);
        }
    }
}
