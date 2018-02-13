use std::str::from_utf8;
use std::process::exit;

use actix::{
    Actor,
    ActorContext,
    Arbiter,
    AsyncContext,
    Context,
    FramedError,
    FramedReader,
    FramedWriter,
    StreamHandler,
    SyncAddress
};
use failure::{Error, ResultExt};
use serde_json::{from_str, Value};
use websocket::ClientBuilder;
use websocket::async::TcpStream;
use websocket::async::futures::{self, Future};
use websocket::codec::ws::MessageCodec;
use websocket::message::OwnedMessage;

use error::ErrorKind;
use interpreter::Interpreter;
use signals::{Event, Link, Rebuild, Receipt, Registration};

pub struct Client {
    pub interpreter: SyncAddress<Interpreter>,
    pub framed: FramedWriter<TcpStream, MessageCodec<OwnedMessage>>,
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
                        let (reader, writer) = FramedReader::wrap(framed);
                        Client::add_stream(reader, ctx);

                        Client {
                            interpreter: interpreter,
                            framed: writer,
                        }
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
}

impl Client {
    /// Process an incoming message on the Websockets connection.
    fn process_message(&mut self, message: OwnedMessage,
                       ctx: &mut Context<Self>) -> Result<(), Error> {
        let contents: String = match message {
            OwnedMessage::Text(m) => m,
            OwnedMessage::Binary(b) => {
                from_utf8(&b).context(ErrorKind::ParseBytesAsUtf8)?.to_string()
            },
            OwnedMessage::Close(_) => {
                info!("received a close from server");
                ctx.stop();
                return Ok(());
            },
            OwnedMessage::Ping(d) => {
                info!("received a ping from server");
                self.framed.send(OwnedMessage::Pong(d));
                return Ok(());
            },
            OwnedMessage::Pong(_) => {
                info!("received a pong from server");
                return Ok(());
            },
        };

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
            "rebuild" => {
                info!("sending rebuild message to interpreter");
                self.interpreter.send(Rebuild {
                    message: contents
                });
                info!("sent rebuild message to interpreter");
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

impl StreamHandler<OwnedMessage, FramedError<MessageCodec<OwnedMessage>>> for Client {
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("websocket client started. sending link to interpreter");
        self.interpreter.send(Link { client: ctx.address() });
    }

    fn handle(&mut self, message: OwnedMessage, ctx: &mut Context<Self>) {
        info!("received message on websockets");
        if let Err(e) = self.process_message(message, ctx) {
            error!("processing message from websockets: error='{}'", e);
        }
    }

    fn finished(&mut self, _: &mut Self::Context) {
        info!("websocket client finished");
        exit(1);
    }
}
