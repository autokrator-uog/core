mod lua;

use std::fs::File;
use std::io::Read;

use actix::{Actor, Address, Context};
use failure::{Error, ResultExt};
use rlua::Lua;

use client::Client;
use error::ErrorKind;
use interpreter::lua::create_lua;

pub const LUA_EVENT_TYPES_REGISTRY_KEY: &'static str = "SERV_EVENT_TYPES";
pub const LUA_CLIENT_TYPE_REGISTRY_KEY: &'static str = "SERV_CLIENT_TYPE";
pub const LUA_EVENT_HANDLER_REGISTRY_KEY: &'static str = "SERV_EVENT_HANDLER";
pub const LUA_RECEIPT_HANDLER_REGISTRY_KEY: &'static str = "SERV_RECEIPT_HANDLER";

pub struct Interpreter {
    pub client: Option<Address<Client>>,
    pub lua: Lua,
}

impl Interpreter {
    pub fn launch(script_path: &str) -> Result<Address<Self>, Error> {
        let mut file = File::open(script_path).context(ErrorKind::LuaScriptNotFound)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).context(ErrorKind::ReadLuaScript)?;

        let interpreter = Self {
            lua: create_lua(contents)?,
            client: None,
        };

        Ok(interpreter.start())
    }
}

impl Actor for Interpreter {
    type Context = Context<Self>;
}
