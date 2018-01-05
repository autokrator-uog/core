use std::net::SocketAddr;

use actix::{Actor, Address, Arbiter, AsyncContext, Context, FramedActor, Handler, StreamHandler};
use actix::{Response, ResponseType};
use bytes::BytesMut;
use failure::{Error, Fail, ResultExt};
use tokio_core::net::TcpStream;
use tokio_io::codec::{Decoder, Encoder, Framed};
use websocket::async::Server as WebsocketServer;
use websocket::async::futures::{Future, Stream};
use websocket::codec::ws::MessageCodec;
use websocket::message::OwnedMessage;
use websocket::server::InvalidConnection;
use websocket::server::upgrade::async::Upgrade;

use bus::Bus;
use error::ErrorKind;
use session::Session;

/// `WsMessage` is a wrapper type that allows us to implement `ResponseType` for Websockets
/// OwnedMessage. It is created by the `Codec` encoder/decoder.
#[derive(Debug)]
pub struct WsMessage(pub OwnedMessage);

impl ResponseType for WsMessage {
    type Item = ();
    type Error = ();
}

/// We create a wrapper around the `MessageCodec` from the websockets library so that we can
/// encode/decode to/from a wrapper message type that implements `ResponseType`.
pub struct Codec {
    inner: MessageCodec<OwnedMessage>,
}

impl Codec {
    /// Create a codec from the `MessageCodec` that we will use for encoding and decoding.
    fn new(message_codec: MessageCodec<OwnedMessage>) -> Codec {
        Codec { inner: message_codec }
    }
}

impl Decoder for Codec {
    type Item = WsMessage;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Decode src using the inner decoder and then wrap the result in a `Message`.
        match self.inner.decode(src).context(ErrorKind::WebsocketCodecWrapperDecoding)? {
            Some(message) => Ok(Some(WsMessage(message))),
            None => Ok(None)
        }
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decode(src)
    }
}

impl Encoder for Codec {
    type Item = WsMessage;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Encode the OwnedMessage from inside the Message into
        // dst using the inner decoder.
        self.inner.encode(item.0, dst).context(
            ErrorKind::WebsocketCodecWrapperEncoding).map_err(Error::from)
    }
}

/// `Connection` is a wrapper type that allows us to implement `ResponseType` for the result of
/// `listener.incoming()` on the websocket server.
pub struct Connection {
    pub upgrade: Upgrade<TcpStream>,
    pub addr: SocketAddr,
}

impl ResponseType for Connection {
    type Item = ();
    type Error = ();
}

/// The server actor handles incoming Websocket connections and creates a session for each
/// incoming connection.
pub struct Server {
    bus: Address<Bus>,
}

impl Server {
    /// Start the websockets server given the arguments for the `server` subcommand.
    pub fn start(bind_addr: &str, bus: Address<Bus>) -> Result<(), Error> {
        // Create a websocket server instance bound to the address provided in arguments.
        let listener = WebsocketServer::bind(bind_addr, Arbiter::handle()).context(
            ErrorKind::UnableToBindWebsocketServer)?;

        info!("starting websocket server on: {}", bind_addr);
        let _: () = Server::create(|ctx| {
            // Add the stream to the server.
            ctx.add_stream(listener.incoming()
               .map_err(|InvalidConnection { error, ..}| {
                   // Wrap error in our own error type.
                   Error::from(error.context(ErrorKind::InvalidWebsocketConnection))
               }).map(|(upgrade, addr)| {
                   // Wrap connections in our wrapper type that implements ResponseType.
                   Connection { upgrade: upgrade, addr: addr }
               }));

            // Return a instance of Server from closure.
            Server { bus: bus }
        });

        Ok(())
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

// By implementing StreamHandler, we can add streams to this actor which will trigger the
// event functions below.
impl StreamHandler<Connection, Error> for Server {
    fn started(&mut self, _ctx: &mut Context<Self>) { info!("websocket server started"); }
    fn finished(&mut self, _ctx: &mut Context<Self>) { info!("websocket server finished"); }
}

impl Handler<Connection, Error> for Server {
    /// Handle an incoming connection and create a session.
    fn handle(&mut self, conn: Connection, _: &mut Context<Self>) -> Response<Self, Connection> {
        // This runs on its own thread so we can just wait on the future from accepting the
        // connection upgrade.
        if let Ok((framed, _)) = conn.upgrade.accept().wait() {
            // For the same reason that we have the Connection type, we need a wrapper type
            // around the MessageCodec from the websockets library so that we can implement
            // ResponseType on OwnedMessage for the Session actor.
            //
            // We do this by splitting the frame into the stream and the codec and create an
            // instance of our wrapper codec before reconstructing the frame.
            let (parts, codec) = framed.into_parts_and_codec();
            let codec = Codec::new(codec);
            let framed = Framed::from_parts(parts, codec);

            // Spawn a session actor from frame and ensure the session has access to the Bus.
            let bus = self.bus.clone();
            let _: () = Session::new(conn.addr, bus).from_framed(framed);
        } else {
            warn!("websocket connection upgrade failed");
        }

        // No need for inter-actor communication so we can return a unit response.
        Self::empty()
    }
}
