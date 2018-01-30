mod lua;

use std::fs::File;
use std::io::Read;

use actix::{Actor, Address, Context};
use failure::{Error, ResultExt};
use rlua::Lua;
use serde_json::from_str;
use vicarius_common::schemas::incoming::NewEventMessage;

use client::Client;
use error::ErrorKind;
use interpreter::lua::create_lua;
use signals::SendMessage;

pub const LUA_EVENT_TYPES_REGISTRY_KEY: &'static str = "SERV_EVENT_TYPES";
pub const LUA_CLIENT_TYPE_REGISTRY_KEY: &'static str = "SERV_CLIENT_TYPE";
pub const LUA_EVENT_HANDLER_REGISTRY_KEY: &'static str = "SERV_EVENT_HANDLER";
pub const LUA_RECEIPT_HANDLER_REGISTRY_KEY: &'static str = "SERV_RECEIPT_HANDLER";

pub struct Interpreter {
    pub client: Address<Client>,
    pub lua: Lua,
}

impl Interpreter {
    pub fn launch(script_path: String, client: Address<Client>) -> Result<Address<Self>, Error> {
        let mut file = File::open(&script_path).context(ErrorKind::LuaScriptNotFound)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).context(ErrorKind::ReadLuaScript)?;

        let interpreter = Self {
            // We use the create_lua function from another module to inject any functions that
            // don't depend on the internal Interpreter state. This helps avoid pesky lifetime
            // issues.
            lua: create_lua()?,
            client: client,
        };

        // The send function needs access to the client field of the interpreter and therefore
        // can't be within the lua module.
        interpreter.inject_send_function()?;

        // For our script to not error, we need to ensure everything is injected before running
        // the script.
        interpreter.lua.eval::<()>(&contents, None).context(ErrorKind::EvaluateLuaScript)?;

        Ok(interpreter.start())
    }

    fn inject_send_function(&self) -> Result<(), Error> {
        // We can't clone the lua object, but we can clone the addresses that we intend to use
        // within the closure.
        let client = self.client.clone();
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
}

impl Actor for Interpreter {
    type Context = Context<Self>;
}
