use failure::{Error, ResultExt};
use rlua::{Error as LuaError, Function, Lua};

use error::ErrorKind;
use interpreter::{
    LUA_CLIENT_TYPE_REGISTRY_KEY,
    LUA_EVENT_HANDLER_REGISTRY_KEY,
    LUA_EVENT_TYPES_REGISTRY_KEY,
    LUA_RECEIPT_HANDLER_REGISTRY_KEY
};

pub fn create_lua(script: String) -> Result<Lua, Error> {
    let lua = Lua::new();

    inject_register_function(&lua)?;
    inject_logging_functions(&lua)?;

    // Run the script provided.
    lua.eval::<()>(&script, None).context(ErrorKind::EvaluateLuaScript)?;
    Ok(lua)
}

fn register(lua: &Lua, (client_type, event_types, new_event_handler, receipt_handler):
            (String, Vec<String>, Function, Function)) -> Result<(), LuaError> {
    info!("received register call from script: client_type='{}'", client_type);

    lua.set_named_registry_value(LUA_EVENT_TYPES_REGISTRY_KEY, event_types)?;
    lua.set_named_registry_value(LUA_CLIENT_TYPE_REGISTRY_KEY, client_type)?;

    // Save our handlers so we can call them with events.
    lua.set_named_registry_value(LUA_EVENT_HANDLER_REGISTRY_KEY, new_event_handler)?;
    lua.set_named_registry_value(LUA_RECEIPT_HANDLER_REGISTRY_KEY, receipt_handler)?;

    Ok(())
}

fn inject_register_function(lua: &Lua) -> Result<(), Error> {
    let globals = lua.globals();

    let register_function = lua.create_function(register).context(
        ErrorKind::CreateRegisterFunction)?;
    globals.set("register", register_function).context(ErrorKind::InjectRegisterFunction)?;
    Ok(())
}

fn inject_logging_functions(lua: &Lua) -> Result<(), Error> {
    let globals = lua.globals();

    let trace_function = lua.create_function(|_, message: String| {
        trace!("from lua: {}", message);
        Ok(())
    }).context(ErrorKind::CreateTraceFunction)?;
    globals.set("trace", trace_function).context(ErrorKind::InjectTraceFunction)?;

    let debug_function = lua.create_function(|_, message: String| {
        debug!("from lua: {}", message);
        Ok(())
    }).context(ErrorKind::CreateDebugFunction)?;
    globals.set("debug", debug_function).context(ErrorKind::InjectDebugFunction)?;

    let info_function = lua.create_function(|_, message: String| {
        info!("from lua: {}", message);
        Ok(())
    }).context(ErrorKind::CreateInfoFunction)?;
    globals.set("info", info_function).context(ErrorKind::InjectInfoFunction)?;

    let warn_function = lua.create_function(|_, message: String| {
        warn!("from lua: {}", message);
        Ok(())
    }).context(ErrorKind::CreateWarnFunction)?;
    globals.set("warn", warn_function).context(ErrorKind::InjectWarnFunction)?;

    let error_function = lua.create_function(|_, message: String| {
        error!("from lua: {}", message);
        Ok(())
    }).context(ErrorKind::CreateErrorFunction)?;
    globals.set("error", error_function).context(ErrorKind::InjectErrorFunction)?;
    Ok(())
}
