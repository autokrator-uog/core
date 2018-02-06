use failure::{Error, ResultExt};
use serde_json::{Value, from_str, to_string};
use rlua::{Lua, Table, ToLua, Value as LuaValue};

use error::ErrorKind;

pub fn json_to_lua<'lua>(lua: &'lua Lua, value: Value) -> Result<Table<'lua>, Error> {
    // Convert the json to a string.
    let as_string: String = to_string(&value).context(ErrorKind::SerializeJsonForLua)?;

    // Decode that into lua.
    let to_eval: String = format!("json.decode({:?})", as_string);
    let as_lua: Table = lua.eval(&to_eval, None).context(ErrorKind::EvaluateJsonToLuaConversion)?;

    Ok(as_lua)
}

pub fn lua_to_json<'lua>(lua: &'lua Lua, table: Table<'lua>) -> Result<Value, Error> {
    // In order to get the string from the table, we need to run the json.encode lua function,
    // that means we need the table as available as a variable. There's no nice way to do this, so
    // we set a global variable that's unlikely to be used.
    let globals = lua.globals();
    globals.set("__ltj_temp", table).context(ErrorKind::SetLuaToJsonTemporary)?;

    // Evaluate the encode function to get the Rust string for the Lua table.
    let as_string: String = lua.eval("json.encode(__ltj_temp)", None).context(
        ErrorKind::EvaluateLuaToJsonConversion)?;

    // Clear the global variable.
    let nil = None::<Option<String>>.to_lua(lua).context(ErrorKind::NilToLuaConversion)?;
    globals.set("__ltj_temp", nil).context(ErrorKind::RemoveLuaToJsonTemporary)?;

    // Convert the JSON string into a Value.
    let value: Value = from_str(&as_string).context(ErrorKind::DeserializeJsonForLua)?;
    Ok(value)
}

pub fn lua_to_string<'lua>(lua: &'lua Lua, value: LuaValue<'lua>) -> Result<String, Error> {
    match value {
        LuaValue::Boolean(val) => Ok(format!("{}", val)),
        LuaValue::Integer(val) => Ok(format!("{}", val)),
        LuaValue::Number(val) => Ok(format!("{}", val)),
        LuaValue::String(val) => {
            let converted = val.to_str().context(ErrorKind::LuaToStringStrConversion)?;
            Ok(String::from(converted))
        },
        LuaValue::Table(val) => {
            let parsed: Value = lua_to_json(lua, val)?;
            Ok(to_string(&parsed).context(ErrorKind::SerializeJsonForLuaToString)?)
        },
        _ => Err(Error::from(ErrorKind::UnsupportedLoggingType)),
    }
}
