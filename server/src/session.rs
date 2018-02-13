use std::str::from_utf8;
use std::net::SocketAddr;

use actix::{
    Actor,
    ActorContext,
    Address,
    AsyncContext,
    Context,
    FramedError,
    FramedWriter,
    StreamHandler
};
use failure::{Error, ResultExt};
use serde_json::{from_str, Value};
use websocket::async::TcpStream;
use websocket::codec::ws::MessageCodec;
use websocket::message::OwnedMessage;

use bus::Bus;
use error::ErrorKind;
use signals;

/// Session contains the state pertaining to one connected client.
pub struct Session {
    pub addr: SocketAddr,
    bus: Address<Bus>,
    pub framed: FramedWriter<TcpStream, MessageCodec<OwnedMessage>>,
    session_id: usize,
}

impl Session {
    /// Create a Session from a socket address and a bus actor.
    pub fn new(addr: SocketAddr, bus: Address<Bus>, session_id: usize,
               framed: FramedWriter<TcpStream, MessageCodec<OwnedMessage>>) -> Self {
        Self {
            addr,
            bus,
            session_id,
            framed,
        }
    }

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
            "query" => {
                info!("sending query message to bus");
                let query = signals::Query {
                    message: contents,
                    sender: ctx.address(),
                    bus: self.bus.clone(),
                };
                self.bus.send(query);
                info!("sent query message to bus");
            },
            "new" => {
                info!("sending new event message to bus");
                let new_event = signals::NewEvent {
                    message: contents,
                    sender: (ctx.address(), self.addr),
                    bus: self.bus.clone(),
                    session_id: self.session_id,
                };
                self.bus.send(new_event);
                info!("sent new event message to bus");
            },
            "register" => {
                info!("sending register message to bus");
                let register = signals::Register {
                    message: contents,
                    sender: (ctx.address(), self.addr),
                    bus: self.bus.clone(),
                };
                self.bus.send(register);
                info!("sent register message to bus");
            },
            "ack" => {
                info!("sending awknowledgement message to bus");
                let awknowledgement = signals::Awknowledgement {
                    message: contents,
                    addr: self.addr,
                };
                self.bus.send(awknowledgement);
                info!("sent awknowledgement message to bus");
            },
            _ => {
                return Err(Error::from(ErrorKind::InvalidWebsocketMessageType));
            },
        };

        Ok(())
    }
}

impl Actor for Session {
    type Context = Context<Self>;
}

impl StreamHandler<OwnedMessage, FramedError<MessageCodec<OwnedMessage>>> for Session {
    // When we get a new session, talk to the bus actor and add it to the sessions map.
    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("started session: client='{}'", self.addr);

        info!("sending connect message to bus");
        let connect = signals::Connect {
            session: ctx.address(),
            addr: self.addr,
        };
        self.bus.send(connect);
        info!("sent connect message to bus");
    }

    fn handle(&mut self, message: OwnedMessage, ctx: &mut Context<Self>) {
        if let Err(e) = self.process_message(message, ctx) {
            error!("processing message from websockets: session='{}' error='{}'", self.addr, e);
        }
    }

    // When we're done with a session, talk to the bus actor and remove it from the sessions map.
    fn finished(&mut self, _: &mut Self::Context) {
        debug!("finished session: client='{}'", self.addr);

        info!("sending disconnect message to bus");
        let disconnect = signals::Disconnect {
            addr: self.addr,
        };
        self.bus.send(disconnect);
        info!("sent connect message to bus");
    }
}
