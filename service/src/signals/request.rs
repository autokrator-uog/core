use actix::{Context, Handler, ResponseType};
use actix_web::Method;
use failure::{Error, Fail, ResultExt};
use http::uri::Uri;
use rlua::{Function, ToLua, Value as LuaValue};
use serde_json::{Value, to_string_pretty};

use error::ErrorKind;
use interpreter::{Bus, Interpreter, json_to_lua, lua_to_json};

pub struct Response(pub u16, pub Option<Value>);

/// The `Request` signal is sent from the http server to the interpreter when a http request is
/// received that needs dealt with.
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub content: Option<Value>,
}

impl ResponseType for Request {
    type Item = Response;
    type Error = Error;
}

impl Interpreter {
    fn handle_request(&mut self, req: Request) -> Result<Response, Error> {
        let uri = format!("{}", req.uri);
        let method = String::from(req.method.as_str());

        let content = if let Some(content) = req.content {
            debug!("handling request from http: uri='{}' method='{}' content=\n{}",
                  uri, method, to_string_pretty(&content)?);
            Some(json_to_lua(&self.lua, content).context(ErrorKind::ParseHttpContent)?)
        } else {
            debug!("handling request from http: uri='{}' method='{}'", uri, method);
            None
        };

        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;

        match bus.http_router.match_route(&uri.clone(), &method.clone()) {
            Some((key, matches)) => {
                let function: Function = self.lua.named_registry_value(&key).context(
                    ErrorKind::MissingHttpHandlerRegistryValue)?;

                let matches_table = match matches.to_lua(&self.lua).context(
                        ErrorKind::MatchesToLua)? {
                    LuaValue::Table(t) => t,
                    _ => {
                        error!("failed to convert pattern matches to lua table");
                        return Err(Error::from(ErrorKind::MatchesToLua));
                    },
                };

                debug!("calling http handler");
                let args = (method, uri, matches_table, content);
                let (status_code, result) = match function.call::<_, (u16, LuaValue)>(args) {
                    Ok((s, r)) => (s, r),
                    Err(e) => {
                        error!("failure running http hander: \n\n{}\n", e);
                        return Err(Error::from(e.context(ErrorKind::FailedHttpHandler)));
                    },
                };

                match result {
                    LuaValue::Nil => {
                        debug!("responding to http request: status_code='{}'", status_code);
                        Ok(Response(status_code, None))
                    },
                    LuaValue::Table(t) => {
                        let value: Value = lua_to_json(&self.lua, t).context(
                            ErrorKind::ParseHttpHandlerResult)?;

                        debug!("responding to http request: status_code='{}' content=\n{}",
                               status_code, to_string_pretty(&value)?);
                        Ok(Response(status_code, Some(value)))
                    },
                    _ => {
                        error!("invalid return type from http handler");
                        Err(Error::from(ErrorKind::HttpHandlerInvalidReturnType))
                    }
                }
            },
            None => {
                warn!("received request for route that could not be found");
                Ok(Response(404, None))
            }
        }
    }
}

impl Handler<Request> for Interpreter {
    type Result = Result<Response, Error>;

    fn handle(&mut self, req: Request, _: &mut Context<Self>) -> Self::Result {
        debug!("received request signal from client");
        self.handle_request(req)
    }
}
