use std::net::SocketAddr;

use actix::{Actor, ActorContext, Address, AsyncContext, FramedActor, FramedContext, Handler};
use actix::{StreamHandler, Response};
use failure::Error;
use tokio_core::net::TcpStream;

use bus::Bus;
use messages;
use server::{Codec, WsMessage};

/// Session contains the state pertaining to one connected client.
pub struct Session {
    addr: SocketAddr,
    bus: Address<Bus>,
}

impl Session {
    /// Create a Session from a socket address and a bus actor.
    pub fn new(addr: SocketAddr, bus: Address<Bus>) -> Session {
        Session {
            addr: addr,
            bus: bus,
        }
    }
}

impl Actor for Session {
    type Context = FramedContext<Self>;
}

impl FramedActor for Session {
    type Io = TcpStream;
    type Codec = Codec;
}

impl StreamHandler<WsMessage, Error> for Session {
    // When we get a new session, talk to the bus actor and add it to the sessions map.
    fn started(&mut self, ctx: &mut FramedContext<Self>) {
        debug!("started handler: {:?}", self.addr);

        info!("sending connect message to bus");
        let connect = messages::Connect {
            session: ctx.address(),
            addr: self.addr,
        };
        self.bus.send(connect);
        info!("sent connect message to bus");
    }

    // When we're done with a session, talk to the bus actor and remove it from the sessions map.
    fn finished(&mut self, _: &mut FramedContext<Self>) {
        debug!("finished handler: {:?}", self.addr);

        info!("sending disconnect message to bus");
        let disconnect = messages::Disconnect {
            addr: self.addr,
        };
        self.bus.send(disconnect);
        info!("sent connect message to bus");
    }
}

impl Handler<WsMessage, Error> for Session {
    fn error(&mut self, err: Error, ctx: &mut FramedContext<Self>) {
        error!("closing session {:?} due to error: {}", self.addr, err);
        ctx.stop()
    }

    fn handle(&mut self, message: WsMessage,
              ctx: &mut FramedContext<Self>) -> Response<Self, WsMessage> {
        info!("session {:?} received message: {:?}", self.addr, message);
        ctx.send(message);
        Self::empty()
    }
}
