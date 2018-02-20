use std::str::from_utf8;

use actix::SyncAddress;
use actix_web::{
    Application,
    AsyncResponder,
    Error as ActixWebError,
    HttpRequest,
    HttpResponse,
    HttpServer,
    StatusCode,
};
use actix_web::httpcodes::{HTTPInternalServerError, HTTPOk};
use failure::{Error, ResultExt};
use serde_json::from_str;
use websocket::async::futures::Future;

use error::ErrorKind;
use interpreter::Interpreter;
use signals::Request;

struct State {
    interpreter: SyncAddress<Interpreter>,
}

fn handle(req: HttpRequest<State>) -> Box<Future<Item=HttpResponse, Error=ActixWebError>> {
    info!("received a request on http");
    let interpreter = req.state().interpreter.clone();
    let method = req.method().clone();
    let uri = req.uri().clone();

    if req.uri() == "/health_check" {
        info!("responding to health check");
        return req.body()
            .from_err()
            .and_then(move |_| {
                Ok(HTTPOk.with_body("OK"))
            }).responder();
    }

    req.body()
       .limit(2048)
       .from_err()
       .and_then(move |bytes| {
           let body: Option<String> = from_utf8(&bytes.to_vec()).ok().map(|val| String::from(val));

           let request = Request {
               method: method,
               uri: uri,
               content: body.and_then(|val| from_str(&val).ok()),
           };

           match interpreter.call_fut(request).wait() {
               Ok(Ok(response)) => {
                   let status_code = match StatusCode::from_u16(response.0) {
                       Ok(code) => code,
                       Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
                   };

                   let mut constructed = HttpResponse::build(status_code);
                   if let Some(value) = response.1 {
                       constructed.json(value)
                   } else {
                       constructed.finish().map_err(ActixWebError::from)
                   }
               },
               Err(e) => {
                   error!("failure in request signal: error='{}'", e);
                   Ok(HTTPInternalServerError.into())
               },
               _ => Ok(HTTPInternalServerError.into()),
           }
       }).responder()
}

pub fn start_webserver(bind_address: String,
                       interpreter: SyncAddress<Interpreter>) -> Result<(), Error> {
    info!("starting http server: bind='{}'", bind_address);
    HttpServer::new(move || {
            let state = State { interpreter: interpreter.clone() };

            Application::with_state(state)
                .handler("/", handle)
        })
        .bind(bind_address)
        .context(ErrorKind::HttpBindToPort)?
        .start();
    Ok(())
}
