use actix::{Context, Handler, ResponseType};
use actix_web::Method;
use failure::{Error, ResultExt};
use http::uri::Uri;
use rlua::Function;
use serde_json::{Value, from_str, to_string};

use error::ErrorKind;
use interpreter::{Bus, Interpreter};

/// The `Request` signal is sent from the http server to the interpreter when a http request is
/// received that needs dealt with.
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub content: Value,
}

impl ResponseType for Request {
    type Item = Value;
    type Error = Error;
}

impl Interpreter {
    fn handle_request(&mut self, req: Request) -> Result<Value, Error> {
        let uri = format!("{}", req.uri);
        let method = String::from(req.method.as_str());
        let content = to_string(&req.content)?;
        info!("handling request from http: uri='{:?}' method='{:?}' content=\n{}",
              uri, method, content);

        let globals = self.lua.globals();
        let bus: Bus = globals.get::<_, Bus>("bus").context(ErrorKind::MissingBusUserData)?;

        match bus.http_handlers.get(&(uri.clone(), method.clone())) {
            Some(key) => {
                let function: Function = self.lua.named_registry_value(key).context(
                    ErrorKind::MissingHttpHandlerRegistryValue)?;

                debug!("calling http handler");
                let args = (method, uri, content);
                let result = function.call::<_, String>(args).context(
                    ErrorKind::FailedHttpHandler)?;

                let value: Value = from_str(&result).context(
                    ErrorKind::ParseHttpHandlerResult)?;

                Ok(value)
            },
            None => return Err(Error::from(ErrorKind::MissingHttpHandlerRegistryValue)),
        }
    }
}

impl Handler<Request> for Interpreter {
    type Result = Result<Value, Error>;

    fn handle(&mut self, req: Request, _: &mut Context<Self>) -> Self::Result {
        debug!("received request signal from client");
        self.handle_request(req)
    }
}
