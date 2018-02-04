use actix::SyncAddress;
use actix_web::{Application, HttpRequest, HttpResponse, HttpServer};
use actix_web::httpcodes::HTTPOk;
use failure::{Error, ResultExt};

use error::ErrorKind;
use interpreter::Interpreter;

struct State {
    interpreter: SyncAddress<Interpreter>,
}

fn handle(_req: HttpRequest<State>) -> HttpResponse {
    info!("got a request");
    HttpResponse::from(HTTPOk)
}

pub fn start_webserver(bind_address: String,
                       interpreter: SyncAddress<Interpreter>) -> Result<(), Error> {
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
