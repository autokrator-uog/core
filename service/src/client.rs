use std::process::exit;

use actix::{Actor, Arbiter, AsyncContext, Context, FramedActor, FramedCell, SyncAddress};
use failure::{Error, ResultExt};
use serde_json::{from_str, Value};
use websocket::{ClientBuilder, WebSocketError};
use websocket::async::TcpStream;
use websocket::async::futures::{self, Future};
use websocket::codec::ws::MessageCodec;
use websocket::message::OwnedMessage;
use vicarius_common::websocket_message_contents;

use error::ErrorKind;
use interpreter::Interpreter;
use signals::{Event, Link, Receipt, Registration};

pub struct Client {
    pub interpreter: SyncAddress<Interpreter>,
    pub framed: Option<FramedCell<Client>>,
}

impl Client {
    pub fn launch(server_address: String,
                  interpreter: SyncAddress<Interpreter>) -> Result<(), Error> {
        Arbiter::handle().spawn(
            ClientBuilder::new(&server_address)
                .context(ErrorKind::WebsocketClientBuilderCreate)?
                .async_connect_insecure(Arbiter::handle())
                .and_then(|(framed, _)| {
                    let _: () = Client::create(|ctx| {
                        let mut client = Client {
                            interpreter: interpreter,
                            framed: None,
                        };

                        let cell = client.add_framed(framed, ctx);
                        client.framed = Some(cell);

                        client
                    });

                    futures::future::ok(())
                })
                .map_err(|e| {
                    error!("closing service, failed to start websocket client: error='{:?}'", e);
                    exit(1);
                })
        );
        Ok(())
    }
}

impl Actor for Client {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("websocket client started. sending link to interpreter");
        self.interpreter.send(Link { client: ctx.address() });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) { info!("websocket client finished"); }
}

impl Client {
    /// Process an incoming message on the Websockets connection.
    fn process_message(&mut self, message: OwnedMessage) -> Result<(), Error> {
        let contents = websocket_message_contents(message).context(
            ErrorKind::InvalidWebsocketMessageType)?;

        let parsed_contents: Value = from_str(&contents).context(
                ErrorKind::ParseJsonFromWebsockets)?;
        let message_type = parsed_contents["message_type"].as_str().ok_or(
                ErrorKind::NoMessageTypeFromWebsockets)?;

        match message_type {
            "event" => {
                info!("sending event message to interpreter");
                self.interpreter.send(Event {
                    message: contents
                });
                info!("sent event message to interpreter");
            },
            "receipt" => {
                info!("sending receipt message to interpreter");
                self.interpreter.send(Receipt {
                    message: contents
                });
                info!("sent receipt message to interpreter");
            },
            "registration" => {
                info!("sending registration message to interpreter");
                self.interpreter.send(Registration {
                    message: contents
                });
                info!("sent registration message to interpreter");
            },
            _ => {
                return Err(Error::from(ErrorKind::InvalidWebsocketMessageType));
            },
        };

        Ok(())
    }
}

impl FramedActor for Client {
    type Io = TcpStream;
    type Codec = MessageCodec<OwnedMessage>;

    fn handle(&mut self, message: Result<OwnedMessage, WebSocketError>,
              _ctx: &mut Context<Self>) {
        info!("received message on websockets");
        let message = match message {
            Ok(m) => m,
            Err(e) => {
                error!("incoming client message: error='{:?}'", e);
                return;
            },
        };

        if let Err(e) = self.process_message(message) {
            match &e.downcast::<ErrorKind>() {
                &Ok(ErrorKind::InvalidWebsocketMessageType) => {
                    warn!("invalid message type: error='{}'",
                           Error::from(ErrorKind::InvalidWebsocketMessageType));
                },
                // Not able to collapse these two conditions into a single condition.
                &Ok(ref e) => {
                    error!("processing message from websockets: error='{}'", e);
                }
                &Err(ref e) => {
                    error!("processing message from websockets: error='{}'", e);
                },
            }
        }
    }
}
