use std::collections::HashMap;

use actix::{Arbiter, SyncAddress};
use failure::{Error, ResultExt};
use rand::{Rng, thread_rng};
use redis::{Client as RedisClient, Commands};
use rlua::{Function, Table, UserData, UserDataMethods, Value as LuaValue};
use serde_json::{Value, from_str, to_string, to_string_pretty};
use websocket::async::futures;

use error::ErrorKind;
use interpreter::Interpreter;
use interpreter::extensions::{ToLuaError, ToLuaErrorResult};
use interpreter::helpers::{json_to_lua, lua_to_json, lua_to_string};
use signals::NewEvent;

#[derive(Clone)]
pub struct Bus {
    pub interpreter: SyncAddress<Interpreter>,
    pub redis: RedisClient,

    pub event_types: Vec<String>,
    pub client_type: Option<String>,

    pub event_handlers: HashMap<String, String>,
    pub receipt_handlers: HashMap<String, String>,
    pub http_handlers: HashMap<(String, String), String>,
}

impl Bus {
    pub fn new(address: SyncAddress<Interpreter>, redis: RedisClient) -> Self {
        Bus {
            interpreter: address,
            redis: redis,
            event_types: Vec::new(),
            client_type: None,
            event_handlers: HashMap::new(),
            receipt_handlers: HashMap::new(),
            http_handlers: HashMap::new(),
        }
    }

    fn generate_key(&self) -> String {
        let mut rng = thread_rng();
        let key: String = rng.gen::<u32>().to_string();
        key
    }
}

impl UserData for Bus {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
        methods.add_method_mut("register", |_, this, client_type: String| {
            debug!("received register call from lua: client_type'{}'", client_type);
            this.client_type = Some(client_type);
            debug!("finished register call from lua");
            Ok(())
        });

        methods.add_method_mut("add_event_listener", |lua, this,
                               (event_type, handler): (String, Function)| {
            debug!("received add_event_listener call from lua: event_type='{}'", event_type);
            this.event_types.push(event_type.clone());

            let key = this.generate_key();
            lua.set_named_registry_value(&key, handler)?;

            match this.event_handlers.insert(event_type.clone(), key) {
                Some(_) => info!("old event handler replaced: type='{}'", event_type),
                None => info!("new event handler added: type='{}'", event_type),
            }
            Ok(())
        });

        methods.add_method_mut("add_receipt_listener", |lua, this,
                               (event_type, handler): (String, Function)| {
            debug!("received add_receipt_listener call from lua: event_type='{}'", event_type);
            let key = this.generate_key();
            lua.set_named_registry_value(&key, handler)?;

            match this.receipt_handlers.insert(event_type.clone(), key) {
                Some(_) => info!("old receipt handler replaced: type='{}'", event_type),
                None => info!("new receipt handler added: type='{}'", event_type),
            }
            Ok(())
        });

        methods.add_method_mut("add_route", |lua, this,
                               (path, method, handler): (String, String, Function)| {
            debug!("received add_route call from lua: path='{}' method='{}'", path, method);
            let key = this.generate_key();
            lua.set_named_registry_value(&key, handler)?;

            let args = (path.clone(), method.clone());
            match this.http_handlers.insert(args, key) {
                Some(_) => info!("old http handler replaced: path='{}' method='{}'", path, method),
                None => info!("new http handler added: path='{}' method='{}'", path, method),
            }
            Ok(())
        });

        methods.add_method("send", |lua, this,
                           (event_type, consistency_key, implicit, correlation_id, data):
                           (String, String, bool, LuaValue, Table)| {
            debug!("received send call from lua");
            let data: Value = lua_to_json(lua, data).to_lua_error()?;

            let correlation_id: Option<usize> = match correlation_id {
                LuaValue::Nil => None,
                LuaValue::Integer(v) => Some(v as usize),
                _ => return Err(Error::from(ErrorKind::InvalidCorrelationIdType).to_lua_error()),
            };

            // In order to use actor addresses, we must be running from an actor, else the program
            // will hang and everything breaks. To get around this, we can use the arbiter handle
            // to send the signal.
            let interpreter = this.interpreter.clone();
            Arbiter::handle().spawn_fn(move || {
                interpreter.send(NewEvent {
                    event_type,
                    consistency_key,
                    data,
                    implicit,
                    correlation_id,
                });
                futures::future::ok(())
            });

            debug!("finished send call from lua");
            Ok(())
        });

        methods.add_method("persist", |lua, this, (key, value): (String, Table)| {
            debug!("received persist call from lua");
            let as_json: Value = lua_to_json(lua, value).to_lua_error()?;

            let serialized = to_string(&as_json).unwrap();
            let pretty_serialized = to_string_pretty(&as_json).unwrap();

            info!("persisting to redis: key='{}' value=\n{}", key, pretty_serialized);
            match this.redis.set::<String, String, String>(
                key, serialized).context(ErrorKind::RedisPersist) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.to_lua_error()),
            }
        });

        methods.add_method("query", |lua, this, key: String| {
            debug!("received query call from lua");

            info!("querying redis: key='{}'", key);
            match this.redis.get::<_, String>(key).context(ErrorKind::RedisQuery) {
                Ok(value) => {
                    let parsed: Value = from_str(&value).to_lua_error()?;
                    Ok(json_to_lua(lua, parsed).to_lua_error()?)
                },
                Err(e) => Err(e.to_lua_error()),
            }
        });

        methods.add_method("trace", |lua, _, message: LuaValue| {
            trace!("from lua: message='{}'", lua_to_string(lua, message).to_lua_error()?);
            Ok(())
        });

        methods.add_method("debug", |lua, _, message: LuaValue| {
            debug!("from lua: message='{}'", lua_to_string(lua, message).to_lua_error()?);
            Ok(())
        });

        methods.add_method("info", |lua, _, message: LuaValue| {
            info!("from lua: message='{}'", lua_to_string(lua, message).to_lua_error()?);
            Ok(())
        });

        methods.add_method("warn", |lua, _, message: LuaValue| {
            warn!("from lua: message='{}'", lua_to_string(lua, message).to_lua_error()?);
            Ok(())
        });

        methods.add_method("error", |lua, _, message: LuaValue| {
            error!("from lua: message='{}'", lua_to_string(lua, message).to_lua_error()?);
            Ok(())
        });

    }
}
