use rlua::{UserData, UserDataMethods, Value as LuaValue};

use interpreter::extensions::ToLuaErrorResult;
use interpreter::helpers::lua_to_string;

#[derive(Clone)]
pub struct Logger { }

impl Logger {
    pub fn new() -> Self {
        Self { }
    }
}

impl UserData for Logger {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
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
