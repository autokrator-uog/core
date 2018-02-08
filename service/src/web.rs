use actix::SyncAddress;
use actix_web::{Application, AsyncResponder, Error as ActixWebError, HttpRequest, HttpResponse, HttpServer};
use actix_web::httpcodes::HTTPInternalServerError;
use failure::{Error, ResultExt};
use serde_json::Value;
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

    req.json()
       .from_err()
       .and_then(move |value: Value| {
           let request = Request {
               method: method,
               uri: uri,
               content: value,
           };

           match interpreter.call_fut(request).wait() {
               Ok(Ok(response)) => Ok(response.to_http_response()?),
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
