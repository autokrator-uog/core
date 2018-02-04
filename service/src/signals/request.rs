use actix::{Context, Handler, ResponseType};
use actix_web::Method;
use failure::{Error, ResultExt};
use http::uri::Uri;
use rlua::Function;
use serde_json::{Value, from_str, to_string, to_string_pretty};

use error::ErrorKind;
use interpreter::{LUA_HTTP_HANDLER_REGISTRY_KEY, Interpreter};

/// The `Link` signal is sent from the client to the interpreter when the client starts so that
/// the register message can be sent.
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
        info!("handling request from http: uri='{:?}' method='{:?}' content=\n{}",
              req.uri, req.method, to_string_pretty(&req.content)?);

        let func: Function = self.lua.named_registry_value(LUA_HTTP_HANDLER_REGISTRY_KEY).context(
            ErrorKind::MissingHttpHandlerRegistryValue)?;

        debug!("calling http handler");
        let uri = format!("{}", req.uri);
        let args = (req.method.as_str(), uri, to_string(&req.content)?);

        let result = func.call::<_, String>(args).context(ErrorKind::FailedHttpHandler)?;
        from_str(&result).context(ErrorKind::ParseHttpHandlerResult).map_err(Error::from)
    }
}

impl Handler<Request> for Interpreter {
    type Result = Result<Value, Error>;

    fn handle(&mut self, req: Request, _: &mut Context<Self>) -> Self::Result {
        debug!("received request signal from client");
        self.handle_request(req)
    }
}
