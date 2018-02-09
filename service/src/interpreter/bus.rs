use std::collections::HashMap;
use std::rc::Rc;

use actix::{Arbiter, SyncAddress};
use failure::Error;
use rand::{Rng, thread_rng};
use rlua::{Function, Table, UserData, UserDataMethods, Value as LuaValue};
use serde_json::Value;
use websocket::async::futures;

use error::ErrorKind;
use interpreter::Interpreter;
use interpreter::extensions::{ToLuaError, ToLuaErrorResult};
use interpreter::helpers::lua_to_json;
use interpreter::router::Router;
use signals::NewEvent;

#[derive(Clone)]
pub struct Bus {
    pub interpreter: SyncAddress<Interpreter>,

    pub event_types: Vec<String>,
    pub client_type: Option<String>,

    pub event_handlers: HashMap<String, String>,
    pub receipt_handlers: HashMap<String, String>,
    pub http_router: Rc<Router>,
}

impl Bus {
    pub fn new(address: SyncAddress<Interpreter>) -> Self {
        Self {
            interpreter: address,
            event_types: Vec::new(),
            client_type: None,
            event_handlers: HashMap::new(),
            receipt_handlers: HashMap::new(),
            http_router: Rc::new(Router::new()),
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

            match Rc::get_mut(&mut this.http_router) {
                Some(router) => {
                    match router.add_route(path.clone(), method.clone(), key) {
                        Ok(Some(_)) => info!("old http handler replaced: path='{}' method='{}'",
                                             path, method),
                        Ok(None) => info!("new http handler added: path='{}' method='{}'",
                                          path, method),
                        Err(e) => error!("failed to add route: error='{}'", e),
                    }
                },
                None => error!("unable to mutably borrow http router"),
            }
            Ok(())
        });

        methods.add_method("send", |lua, this,
                           (event_type, consistency_key, implicit, correlation_id, data):
                           (String, String, bool, LuaValue, Table)| {
            debug!("received send call from lua");
            let data: Value = lua_to_json(lua, data).to_lua_error()?;

            let correlation_id: Option<u32> = match correlation_id {
                LuaValue::Nil => None,
                LuaValue::Integer(v) => Some(v as u32),
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


    }
}
