use std::fmt::Display;
use std::sync::Arc;

use failure::{Context, Error as FailureError};
use serde_json::Error as JsonError;
use rlua::Error as LuaError;

pub trait ToLuaError {
    fn to_lua_error(self) -> LuaError;
}

impl ToLuaError for FailureError {
    fn to_lua_error(self) -> LuaError {
        LuaError::ExternalError(Arc::new(self))
    }
}

impl ToLuaError for JsonError {
    fn to_lua_error(self) -> LuaError {
        LuaError::ExternalError(Arc::new(FailureError::from(self)))
    }
}

impl<E: Display + Send + Sync> ToLuaError for Context<E> {
    fn to_lua_error(self) -> LuaError {
        LuaError::ExternalError(Arc::new(FailureError::from(self)))
    }
}

pub trait ToLuaErrorResult<T> {
    fn to_lua_error(self) -> Result<T, LuaError>;
}

impl<T, E> ToLuaErrorResult<T> for Result<T, E>
    where E: ToLuaError
{
    fn to_lua_error(self) -> Result<T, LuaError> {
        self.map_err(|e| e.to_lua_error())
    }
}

