use std::net::SocketAddr;
use std::str::from_utf8;

use actix::{Actor, Address, AsyncContext, Context, FramedActor, FramedCell};
use failure::{Error, ResultExt};
use serde_json::{from_str, Value};
use tokio_core::net::TcpStream;
use websocket::message::OwnedMessage;

use bus::Bus;
use error::ErrorKind;
use server::{Codec, WsMessage};
use signals;

/// Session contains the state pertaining to one connected client.
pub struct Session {
    pub addr: SocketAddr,
    bus: Address<Bus>,
    pub framed: FramedCell<TcpStream, Codec>,
    session_id: usize,
}

impl Session {
    /// Create a Session from a socket address and a bus actor.
    pub fn new(addr: SocketAddr, bus: Address<Bus>, session_id: usize,
               framed: FramedCell<TcpStream, Codec>) -> Self {
        Self {
            addr,
            bus,
            session_id,
            framed,
        }
    }

    /// Process an incoming message on the Websockets connection.
    fn process_message(&mut self, message: WsMessage,
                       ctx: &mut Context<Self>) -> Result<(), Error> {
        let contents = self.get_message_contents(message)?;

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
            _ => {
                return Err(Error::from(ErrorKind::InvalidWebsocketMessageType));
            },
        };

        Ok(())
    }

    /// Returns the contents of a message from a Websockets message.
    fn get_message_contents(&mut self, message: WsMessage) -> Result<String, Error> {
        match message {
            WsMessage(OwnedMessage::Text(m)) => Ok(m),
            WsMessage(OwnedMessage::Binary(b)) => {
                Ok(from_utf8(&b).context(ErrorKind::ParseBytesAsUtf8)?.to_string())
            },
            WsMessage(OwnedMessage::Close(_)) => {
                // We don't stop the actor here as this causes issues with Actix. The stream
                // closes itself in a moment.
                info!("client has disconnected, stream will close and session will be removed \
                       momentarily: client='{}'", self.addr);
                Err(Error::from(ErrorKind::InvalidWebsocketMessageType))
            },
            WsMessage(OwnedMessage::Ping(d)) => {
                self.framed.send(WsMessage(OwnedMessage::Pong(d)));
                Err(Error::from(ErrorKind::InvalidWebsocketMessageType))
            }
            WsMessage(OwnedMessage::Pong(d)) => {
                self.framed.send(WsMessage(OwnedMessage::Ping(d)));
                Err(Error::from(ErrorKind::InvalidWebsocketMessageType))
            }
        }
    }

}

impl Actor for Session {
    type Context = Context<Self>;

    // When we get a new session, talk to the bus actor and add it to the sessions map.
    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("started session: client='{}'", self.addr);

        info!("sending connect message to bus");
        let connect = signals::Connect {
            session: ctx.address(),
            addr: self.addr,
        };
        self.bus.send(connect);
        info!("sent connect message to bus");
    }

    // When we're done with a session, talk to the bus actor and remove it from the sessions map.
    fn stopped(&mut self, _: &mut Context<Self>) {
        debug!("finished session: client='{}'", self.addr);

        info!("sending disconnect message to bus");
        let disconnect = signals::Disconnect {
            addr: self.addr,
        };
        self.bus.send(disconnect);
        info!("sent connect message to bus");
    }
}

impl FramedActor<TcpStream, Codec> for Session {

    fn handle(&mut self, message: Result<WsMessage, Error>,
              ctx: &mut Context<Self>) {
        info!("received message on websockets: client='{}'", self.addr);
        let message = match message {
            Ok(m) => m,
            Err(e) => {
                error!("error on session: error=`{:?}`", e);
                return;
            },
        };

        if let Err(e) = self.process_message(message, ctx) {
            match &e.downcast::<ErrorKind>() {
                &Ok(ErrorKind::InvalidWebsocketMessageType) => {
                    warn!("invalid message type: session='{}' error='{}'",
                           self.addr, Error::from(ErrorKind::InvalidWebsocketMessageType));
                },
                // Not able to collapse these two conditions into a single condition.
                &Ok(ref e) => {
                    error!("processing message from websockets: session='{}' error='{}'",
                           self.addr, e);
                }
                &Err(ref e) => {
                    error!("processing message from websockets: session='{}' error='{}'",
                           self.addr, e);
                },
            }
        }
    }
}
