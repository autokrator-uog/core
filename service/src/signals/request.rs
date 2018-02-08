use actix::{Context, Handler, ResponseType};
use actix_web::{Error as WebError, HttpResponse, Method};
use actix_web::httpcodes::{HTTPNotFound, HTTPOk};
use failure::{Error, Fail, ResultExt};
use http::uri::Uri;
use rlua::{Function, Value as LuaValue};
use serde_json::{Value, to_string_pretty};

use error::ErrorKind;
use interpreter::{Bus, Interpreter, json_to_lua, lua_to_json};

pub enum Response {
    NotFound,
    Empty,
    NonEmpty(Value),
}

impl Response {
    pub fn to_http_response(self) -> Result<HttpResponse, WebError> {
        match self {
            Response::NotFound => Ok(HTTPNotFound.into()),
            Response::Empty => Ok(HTTPOk.into()),
            Response::NonEmpty(val) => HTTPOk.build().json(val),
        }
    }
}

/// The `Request` signal is sent from the http server to the interpreter when a http request is
/// received that needs dealt with.
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub content: Value,
}

impl ResponseType for Request {
    type Item = Response;
    type Error = Error;
}

impl Interpreter {
    fn handle_request(&mut self, req: Request) -> Result<Response, Error> {
        let uri = format!("{}", req.uri);
        let method = String::from(req.method.as_str());
        info!("handling request from http: uri='{:?}' method='{:?}' content=\n{}",
              uri, method, to_string_pretty(&req.content)?);
        let content = json_to_lua(&self.lua, req.content).context(
            ErrorKind::ParseHttpContent)?;

        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;

        match bus.http_handlers.get(&(uri.clone(), method.clone())) {
            Some(key) => {
                let function: Function = self.lua.named_registry_value(key).context(
                    ErrorKind::MissingHttpHandlerRegistryValue)?;

                debug!("calling http handler");
                let args = (method, uri, content);
                let result = match function.call::<_, LuaValue>(args) {
                    Ok(r) => r,
                    Err(e) => {
                        error!("failure running http hander: \n\n{}\n", e);
                        return Err(Error::from(e.context(ErrorKind::FailedHttpHandler)));
                    },
                };

                match result {
                    LuaValue::Nil => {
                        debug!("responding to http request with empty response");
                        Ok(Response::Empty)
                    },
                    LuaValue::Table(t) => {
                        let value: Value = lua_to_json(&self.lua, t).context(
                            ErrorKind::ParseHttpHandlerResult)?;

                        debug!("responding to http request: content=\n{}",
                               to_string_pretty(&value)?);
                        Ok(Response::NonEmpty(value))
                    },
                    _ => {
                        error!("invalid return type from http handler");
                        Err(Error::from(ErrorKind::HttpHandlerInvalidReturnType))
                    }
                }
            },
            None => {
                warn!("received request for route that could not be found");
                Ok(Response::NotFound)
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
