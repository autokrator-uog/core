mod extensions;
mod functions;
mod helpers;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use actix::{Actor, Context, SyncAddress};
use common::schemas::{ConsistencyKey, ConsistencyValue};
use failure::{Error, ResultExt};
use rand::{self, ThreadRng};
use redis::Client as RedisClient;
use rlua::Lua;

use client::Client;
use error::ErrorKind;
pub use interpreter::functions::Bus;
pub use interpreter::helpers::{json_to_lua, lua_to_json};

pub struct Interpreter {
    pub client: Option<SyncAddress<Client>>,
    pub consistency: HashMap<ConsistencyKey, ConsistencyValue>,
    pub lua: Lua,
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

        let interpreter = Self {
            client: None,
            consistency: HashMap::new(),
            lua: Lua::new(),
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
