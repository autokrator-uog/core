mod lua;

use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use actix::{Actor, Context, SyncAddress};
use failure::{Error, ResultExt};
use redis::{Client as RedisClient, Commands, RedisResult};
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
    pub redis: RedisClient,
    pub client: Option<SyncAddress<Client>>,
    pub lua: Lua,
    pub script: String,
}

impl Interpreter {
    pub fn new(script_path: String, redis_address: String) -> Result<Self, Error> {
        let mut file = File::open(&script_path).context(ErrorKind::LuaScriptNotFound)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).context(ErrorKind::ReadLuaScript)?;

        let redis = RedisClient::open(redis_address.deref()).context(
            ErrorKind::RedisClientCreate)?;

        let interpreter = Self {
            // We use the create_lua function from another module to inject any functions that
            // don't depend on the internal Interpreter state. This helps avoid pesky lifetime
            // issues.
            lua: create_lua()?,
            client: None,
            script: contents,
            redis: redis,
        };

        interpreter.inject_query_function()?;
        interpreter.inject_persist_function()?;

        Ok(interpreter)
    }

    fn inject_query_function(&self) -> Result<(), Error> {
        // We can't clone the lua object, but we can clone the addresses that we intend to use
        // within the closure.
        let redis = self.redis.clone();
        let globals = self.lua.globals();

        let query_function = self.lua.create_function(move |_, key: String| {
            info!("received query call from script: query='{}'", key);
            let query_result: RedisResult<String> = redis.get(key);
            match query_result {
                Ok(value) => {
                    debug!("value returned from redis query: value='{}'", value);
                    Ok(Some(value))
                },
                Err(e) => {
                    warn!("error returned from redis query: error='{:?}'", e);
                    Ok(None)
                },
            }
        }).context(ErrorKind::CreateQueryFunction)?;
        globals.set("query", query_function).context(ErrorKind::InjectQueryFunction)?;

        Ok(())
    }

    fn inject_persist_function(&self) -> Result<(), Error> {
        // We can't clone the lua object, but we can clone the addresses that we intend to use
        // within the closure.
        let redis = self.redis.clone();
        let globals = self.lua.globals();

        let persist_function = self.lua.create_function(move |_, (key, value): (String, String)| {
            info!("received persist call from script: key='{}' value='{}'", key, value);
            let persist_result: RedisResult<String> = redis.set(key, value);
            match persist_result {
                Ok(value) => {
                    debug!("value persisted to redis: result='{}'", value);
                },
                Err(e) => {
                    error!("error returned from redis set: error='{:?}'", e);
                },
            }

            Ok(())
        }).context(ErrorKind::CreatePersistFunction)?;
        globals.set("persist", persist_function).context(ErrorKind::InjectPersistFunction)?;

        Ok(())
    }

}

impl Actor for Interpreter {
    type Context = Context<Self>;
}
