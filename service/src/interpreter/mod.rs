mod bus;
mod extensions;
mod helpers;
mod logger;
mod redis;
mod router;
mod status_codes;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use actix::{Actor, Context, SyncAddress};
use common::schemas::{ConsistencyKey, ConsistencyValue, NewEvent};
use failure::{Error, ResultExt};
use rand::{self, ThreadRng};
use redis::Client as RedisClient;
use rlua::Lua;

use client::Client;
use error::ErrorKind;
pub use interpreter::bus::Bus;
pub use interpreter::helpers::{json_to_lua, lua_to_json};
pub use interpreter::logger::Logger;
pub use interpreter::redis::RedisInterface;
pub use interpreter::status_codes::add_http_status_codes;

pub const TIMESTAMP_KEY: &'static str = "__CLIENT_TIMESTAMP";

pub struct Interpreter {
    pub client: Option<SyncAddress<Client>>,
    pub consistency: HashMap<ConsistencyKey, ConsistencyValue>,
    pub lua: Lua,
    pub receipt_lookup: HashMap<String, NewEvent>,
    pub redis: RedisClient,
    pub rng: RefCell<ThreadRng>,
    pub script: String,
}

impl Interpreter {
    pub fn launch(script_path: String, redis_address: String) -> Result<SyncAddress<Self>, Error> {
        let mut file = File::open(&script_path).context(ErrorKind::LuaScriptNotFound)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).context(ErrorKind::ReadLuaScript)?;

        let redis = RedisClient::open(redis_address.deref()).context(
            ErrorKind::RedisClientCreate)?;

        let mut lua = Lua::new();
        add_http_status_codes(&mut lua)?;

        let interpreter = Self {
            client: None,
            consistency: HashMap::new(),
            lua: lua,
            receipt_lookup: HashMap::new(),
            redis: redis,
            rng: RefCell::new(rand::thread_rng()),
            script: contents,
        };

        Ok(interpreter.start())
    }

}

impl Actor for Interpreter {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) { info!("interpreter started"); }
    fn stopping(&mut self, _: &mut Self::Context) -> bool { info!("interpreter stopping"); true }
    fn stopped(&mut self, _: &mut Self::Context) { info!("interpreter finished"); }
}
