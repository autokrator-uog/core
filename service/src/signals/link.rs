use std::process::exit;

use actix::{AsyncContext, Context, Handler, SyncAddress, ResponseType};
use chrono::{DateTime, NaiveDateTime, Utc};
use common::schemas::{Register, Query};
use failure::{Error, ResultExt};
use redis::Commands;
use rlua::Table;

use client::Client;
use error::ErrorKind;
use interpreter::{TIMESTAMP_KEY, Bus, Interpreter, Logger, RedisInterface};
use signals::SendMessage;

static LUA_LIBRARY: &'static str = include_str!("../../vendor/json.lua");

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
    fn load_library(&mut self, name: &str, contents: &'static str) -> Result<(), Error> {
        let globals = self.lua.globals();

        info!("evaluating lua library");
        let script_result: Table = self.lua.exec(&contents, Some(name)).context(
            ErrorKind::EvaluateLuaScript)?;

        info!("setting global: name='{}'", name);
        globals.set(name, script_result)?;
        Ok(())
    }

    fn send_query_for_timestamp(&mut self, client: &SyncAddress<Client>) -> Result<(), Error> {
        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;

        let value = match self.redis.get::<_, i64>(TIMESTAMP_KEY) {
            Ok(v) => v,
            Err(_) => 0,
        };
        let as_datetime = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(value, 0), Utc);
        debug!("querying for timestamp: value='{}' corresponding_date='{}'", value, as_datetime);

        client.send(SendMessage(Query {
            event_types: bus.event_types,
            since: as_datetime.to_rfc3339(),
            message_type: String::from("query"),
        }));

        Ok(())
    }

    fn send_register_message(&mut self, client: &SyncAddress<Client>) -> Result<(), Error> {
        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;

        let event_types = bus.event_types;
        let client_type = bus.client_type.ok_or(ErrorKind::ClientNotLinkedToInterpreter)?;

        info!("sending register message to server: event_types='{:?}' client_type='{}'",
              event_types, client_type);
        client.send(SendMessage(Register {
            client_type,
            event_types,
            message_type: String::from("register"),
        }));
        Ok(())
    }

    fn link_client(&mut self, client: SyncAddress<Client>,
                   ctx: &mut Context<Self>) -> Result<(), Error> {
        // Set the client field on the interpreter.
        self.client = Some(client.clone());

        // Load JSON library.
        self.load_library("json", LUA_LIBRARY)?;

        {
            let redis = self.redis.clone();
            debug!("injecting bus userdata");
            let globals = self.lua.globals();
            globals.set("bus", Bus::new(ctx.address()))?;
            globals.set("log", Logger::new())?;
            globals.set("redis", RedisInterface::new(redis))?;
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
        self.send_register_message(&client)?;

        // Then query for rebuilding.
        self.send_query_for_timestamp(&client)
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
