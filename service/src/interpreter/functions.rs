use std::collections::HashMap;
use std::sync::Arc;

use actix::{Arbiter, SyncAddress};
use failure::{Error, ResultExt};
use rand::{Rng, thread_rng};
use redis::{Client as RedisClient, Commands};
use rlua::{Error as LuaError, Function, ToLua, UserData, UserDataMethods};
use serde_json::{Value, from_str, to_string, to_string_pretty};
use websocket::async::futures;

use error::ErrorKind;
use interpreter::Interpreter;
use signals::SendMessage;

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

        methods.add_method("send", |_, this, value: String| {
            debug!("received send call from lua");
            let parsed: Value = from_str(&value).unwrap();
            let interpreter = this.interpreter.clone();

            // In order to use actor addresses, we must be running from an actor, else the program
            // will hang and everything breaks. To get around this, we can use the arbiter handle
            // to send the signal.
            Arbiter::handle().spawn_fn(move || {
                interpreter.send(SendMessage(parsed));
                futures::future::ok(())
            });
            debug!("finished send call from lua");
            Ok(())
        });

        methods.add_method("persist", |_, this, (key, value): (String, String)| {
            debug!("received persist call from lua");
            let serialized = to_string(&value).unwrap();
            let pretty_serialized = to_string_pretty(&value).unwrap();

            info!("persisting to redis: key='{}' value=\n{}", key, pretty_serialized);
            match this.redis.set::<String, String, String>(
                key, serialized).context(ErrorKind::RedisPersist) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(LuaError::ExternalError(Arc::new(Error::from(e)))),
            }
        });

        methods.add_method("query", |lua, this, key: String| {
            debug!("received query call from lua");

            info!("querying redis: key='{}'", key);
            match this.redis.get::<_, String>(key).context(ErrorKind::RedisQuery) {
                Ok(value) => Ok(value.to_lua(lua)),
                Err(e) => Err(LuaError::ExternalError(Arc::new(Error::from(e)))),
            }
        });

        methods.add_method("trace", |_, _, message: String| {
            trace!("from lua: message='{}'", message);
            Ok(())
        });

        methods.add_method("debug", |_, _, message: String| {
            debug!("from lua: message='{}'", message);
            Ok(())
        });

        methods.add_method("info", |_, _, message: String| {
            info!("from lua: message='{}'", message);
            Ok(())
        });

        methods.add_method("warn", |_, _, message: String| {
            warn!("from lua: message='{}'", message);
            Ok(())
        });

        methods.add_method("error", |_, _, message: String| {
            error!("from lua: message='{}'", message);
            Ok(())
        });

    }
}
