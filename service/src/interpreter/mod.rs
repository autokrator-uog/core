mod lua;

use std::fs::File;
use std::io::Read;

use actix::{Actor, Context, SyncAddress};
use failure::{Error, ResultExt};
use rlua::Lua;

use client::Client;
use error::ErrorKind;
use interpreter::lua::create_lua;

pub const LUA_EVENT_TYPES_REGISTRY_KEY: &'static str = "SERV_EVENT_TYPES";
pub const LUA_CLIENT_TYPE_REGISTRY_KEY: &'static str = "SERV_CLIENT_TYPE";
pub const LUA_HTTP_HANDLER_REGISTRY_KEY: &'static str = "SERV_HTTP_HANDLER";
pub const LUA_EVENT_HANDLER_REGISTRY_KEY: &'static str = "SERV_EVENT_HANDLER";
pub const LUA_RECEIPT_HANDLER_REGISTRY_KEY: &'static str = "SERV_RECEIPT_HANDLER";

pub struct Interpreter {
    pub client: Option<SyncAddress<Client>>,
    pub lua: Lua,
    pub script: String,
}

impl Interpreter {
    pub fn new(script_path: String) -> Result<Self, Error> {
        let mut file = File::open(&script_path).context(ErrorKind::LuaScriptNotFound)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).context(ErrorKind::ReadLuaScript)?;

        let interpreter = Self {
            // We use the create_lua function from another module to inject any functions that
            // don't depend on the internal Interpreter state. This helps avoid pesky lifetime
            // issues.
            lua: create_lua()?,
            client: None,
            script: contents,
        };

        Ok(interpreter)
    }

}

impl Actor for Interpreter {
    type Context = Context<Self>;
}
