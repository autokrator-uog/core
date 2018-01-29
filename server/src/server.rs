use std::cell::RefCell;
use std::net::SocketAddr;

use actix::{Actor, Address, Arbiter, AsyncContext, Context, FramedActor, Handler, ResponseType};
use failure::{Error, Fail, ResultExt};
use rand::{self, Rng, ThreadRng};
use websocket::async::{Server as WebsocketServer, TcpStream};
use websocket::async::futures::{Future, Stream};
use websocket::server::InvalidConnection;
use websocket::server::upgrade::async::Upgrade;

use bus::Bus;
use error::ErrorKind;
use session::Session;

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
    rng: RefCell<ThreadRng>,
}

impl Server {
    /// Start the websockets server given the arguments for the `server` subcommand.
    pub fn launch(bind_addr: &str, bus: Address<Bus>) -> Result<(), Error> {
        // Create a websocket server instance bound to the address provided in arguments.
        let listener = WebsocketServer::bind(bind_addr, Arbiter::handle()).context(
            ErrorKind::UnableToBindWebsocketServer)?;

        info!("starting websocket server on: address='{}'", bind_addr);
        let _: () = Self::create(|ctx| {
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
            Self {
                bus: bus,
                rng: RefCell::new(rand::thread_rng()),
            }
        });

        Ok(())
    }
}

impl Actor for Server {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) { info!("websocket server started"); }
    fn stopped(&mut self, _ctx: &mut Context<Self>) { info!("websocket server finished"); }
}

impl Handler<Result<Connection, Error>> for Server {
    type Result = ();

    /// Handle an incoming connection and create a session.
    fn handle(&mut self, conn: Result<Connection, Error>, _: &mut Context<Self>) {
        // This runs on its own thread so we can just wait on the future from accepting the
        // connection upgrade.
        let conn = match conn {
            Ok(c) => c,
            Err(e) => {
                error!("invalid websocket connection: error='{:?}'", e);
                return;
            },
        };

        if let Ok((framed, _)) = conn.upgrade.accept().wait() {
            // Spawn a session actor from frame and ensure the session has access to the Bus.
            let bus = self.bus.clone();
            let session_id = self.rng.borrow_mut().gen::<usize>();
            let addr = conn.addr;
            let _: () = Session::create_with(framed, move |_, framed| {
                Session::new(addr.clone(), bus.clone(), session_id.clone(), framed)
            });
        } else {
            warn!("websocket connection upgrade failed");
        }
    }
}
