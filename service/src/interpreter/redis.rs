use failure::ResultExt;
use redis::{Client as RedisClient, Commands};
use rlua::{Table, UserData, UserDataMethods};
use serde_json::{Value, from_str, to_string, to_string_pretty};

use error::ErrorKind;
use interpreter::extensions::{ToLuaError, ToLuaErrorResult};
use interpreter::helpers::{json_to_lua, lua_to_json};

#[derive(Clone)]
pub struct RedisInterface {
    pub redis: RedisClient,
}

impl RedisInterface {
    pub fn new(redis: RedisClient) -> Self {
        Self { redis }
    }
}

impl UserData for RedisInterface {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
        methods.add_method("set", |lua, this, (key, value): (String, Table)| {
            debug!("received set call from lua");
            let as_json: Value = lua_to_json(lua, value).to_lua_error()?;

            let serialized = to_string(&as_json).to_lua_error()?;
            let pretty_serialized = to_string_pretty(&as_json).to_lua_error()?;

            info!("persisting to redis: key='{}' value=\n{}", key, pretty_serialized);
            match this.redis.set::<String, String, String>(
                key, serialized).context(ErrorKind::RedisPersist) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.to_lua_error()),
            }
        });

        methods.add_method("get", |lua, this, key: String| {
            debug!("received get call from lua");

            info!("querying redis: key='{}'", key);
            match this.redis.get::<_, String>(key).context(ErrorKind::RedisQuery) {
                Ok(value) => {
                    let parsed: Value = from_str(&value).to_lua_error()?;
                    Ok(json_to_lua(lua, parsed).to_lua_error()?)
                },
                Err(e) => Err(e.to_lua_error()),
            }
        });

        methods.add_method("keys", |lua, this, key: String| {
            debug!("received keys call from lua");

            info!("querying redis for keys: key='{}'", key);
            match this.redis.keys::<_, Vec<String>>(key).context(ErrorKind::RedisKeys) {
                Ok(matching_keys) => {
                    let as_string = to_string(&matching_keys).to_lua_error()?;
                    let parsed: Value = from_str(&as_string).to_lua_error()?;
                    Ok(json_to_lua(lua, parsed).to_lua_error()?)
                },
                Err(e) => Err(e.to_lua_error()),
            }
        });
    }
}
