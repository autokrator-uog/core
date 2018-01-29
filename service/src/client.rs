use actix::{Actor, Address, Arbiter, Context, FramedActor, FramedCell};
use failure::{Error, ResultExt};
use websocket::{ClientBuilder, WebSocketError};
use websocket::async::TcpStream;
use websocket::async::futures::{self, Future};
use websocket::codec::ws::MessageCodec;
use websocket::message::OwnedMessage;

use error::ErrorKind;
use interpreter::Interpreter;

pub struct Client {
    interpreter: Address<Interpreter>,
    framed: FramedCell<TcpStream, MessageCodec<OwnedMessage>>,
}

impl Client {
    /// Start the websockets server given the arguments for the `server` subcommand.
    pub fn launch(server_address: &str, interpreter: Address<Interpreter>) -> Result<(), Error> {
        info!("starting websocket client: server='{}'", server_address);
        Arbiter::handle().spawn(
            ClientBuilder::new(server_address)
                .context(ErrorKind::WebsocketClientBuilderCreate)?
                .async_connect_insecure(Arbiter::handle())
                .and_then(|(framed, _)| {
                    let _: () = Client::create_with(framed, move |_, framed| {
                        Self { interpreter: interpreter, framed: framed }
                    });

                    futures::future::ok(())
                })
                .map_err(|e| {
                    error!("starting websocket client: error='{:?}'", e);
                })
        );
        Ok(())
    }
}

impl Actor for Client {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) { info!("websocket client started"); }
    fn stopped(&mut self, _ctx: &mut Context<Self>) { info!("websocket client finished"); }
}

impl FramedActor<TcpStream, MessageCodec<OwnedMessage>> for Client {
    fn handle(&mut self, message: Result<OwnedMessage, WebSocketError>, _ctx: &mut Context<Self>) {
        info!("received message on websockets");
        let message = match message {
            Ok(m) => m,
            Err(e) => {
                error!("incoming client message: error='{:?}'", e);
                return;
            },
        };

        info!("message: {:?}", message);
    }
}
