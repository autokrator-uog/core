mod functions;

use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use actix::{Actor, Context, SyncAddress};
use failure::{Error, ResultExt};
use redis::Client as RedisClient;
use rlua::Lua;

use client::Client;
use error::ErrorKind;
pub use interpreter::functions::Bus;

pub struct Interpreter {
    pub redis: RedisClient,
    pub client: Option<SyncAddress<Client>>,
    pub lua: Lua,
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
            lua: Lua::new(),
            client: None,
            script: contents,
            redis: redis,
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
