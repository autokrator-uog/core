use err::{ErrorKind, Result};
use failure::{Fail, ResultExt};

use futures::{Future, Sink, Stream};
use tokio_core::reactor::{Handle, Remote, Core};

use common::Event;
use state::{EventLoopState, ServerState};
use consumer;
use producer;

use std::fmt::Debug;
use std::str::from_utf8;

use rdkafka::Message;
use rdkafka::message::OwnedMessage as KafkaOwnedMessage;

use websocket::message::OwnedMessage as WsOwnedMessage;
use websocket::async::Server;
use websocket::server::InvalidConnection;

pub fn bootstrap(bind: &str, brokers: &str, group: &str, topic: &str) -> Result<()> {
    // Create our event loop.
    let mut core = Core::new().context(ErrorKind::EventLoopCreationFailure)?;

    // Create a handle for spawning futures from the main thread.
    let handle = core.handle();
    let remote = core.remote();

    // Create all of the required state.
    let ServerState {state, receive_channel_in, send_channel_in, consumer } =
        ServerState::new(brokers, group, topic)?;

    // Create the websocket server and add it on the event loop.
    let server = Server::bind(bind, &handle).context(ErrorKind::WebsocketServerBindingFailure)?;
    info!("Running websocket server on {:?}", bind);

    // We must clone the state into state_inner so that when it is captured by the
    // move closure, we still have the state for use outside. We will see this pattern
    // throughout.
    let state_inner = state.clone();
    let connection_handler = server.incoming()
        .map_err(|InvalidConnection { error, .. }| error.context(ErrorKind::InvalidConnection))
        .for_each(move |(upgrade, addr)| {
            let state = state_inner.clone();
            let handle_inner = handle.clone();

            // Accept the websocket connection.
            let f = upgrade.accept().and_then(move |(framed, _)| {
                // Split the channels up into send (sink) and receive (stream).
                let (sink, stream) = framed.split();
                // Add the (addr, stream) mapping to the receive_channel_out. We'll
                // be able to loop over receive_channel_in to spawn threads that will
                // handle the incoming messages.
                info!("Accepted connection from {:?}", addr);
                let f = state.receive_channel_out.send((format!("{:?}", addr), stream));
                spawn_future(f, "Send stream to connection pool", &handle_inner);

                // Add the sink to the HashMap so that we can get it when we want
                // to send a message to the client.
                match state.connections.try_write() {
                    Ok(mut m) => {
                        m.insert(format!("{:?}", addr), sink);
                        ()
                    },
                    Err(_) => warn!("RwLock has been poisoned!"),
                }
                Ok(())
            });

            spawn_future(f, "Handle new connection", &handle);
            Ok(())
        });

    let state_inner = state.clone();
    let remote_inner = remote.clone();
    // Spawn a handler for outgoing clients so that this doesn't block the main thread.
    let receive_handler = state.cpu_pool.spawn_fn(|| {
        receive_channel_in.for_each(move |(addr, stream)| {
            let state = state_inner.clone();
            let remote = remote_inner.clone();

            // For each client, spawn a new thread that will process everything we receive from
            // it.
            remote.spawn(move |handle| {
                let state = state.clone();
                let remote = handle.remote().clone();
                let addr = addr.clone();

                stream.for_each(move |msg| {
                    let state = state.clone();
                    let remote = remote.clone();
                    let addr = addr.clone();

                    if let WsOwnedMessage::Close(_) = msg {
                        info!("Removing connection {:?} from clients", &addr);
                        match state.connections.try_write() {
                            Ok(mut m) => {
                                if let None = m.remove(&addr) {
                                    warn!("Unable to remove connection from map");
                                }
                            },
                            Err(_) => warn!("RwLock has been poisoned!"),
                        }
                    } else {
                        remote.spawn(move |_| {
                            info!("Parsing producer message from {:?}", addr);
                            let parsed_message = producer::parse_message(msg);
                            match parsed_message {
                                Ok(m) => {
                                    info!("Processing producer message from {:?}", addr);
                                    if let Err(e) = m.process(addr, &state) {
                                        error!("Error occurred when processing message: {:?}", e);
                                    }
                                },
                                Err(e) => {
                                    warn!("Unable to parse message from websocket: {:?}", e);
                                }
                            }
                            Ok(())
                        });
                    }

                    Ok(())
                }).map_err(|e| {
                    error!("Error in websocket stream: {:?}", e);
                    ()
                })
            });

            Ok(())
        })
    });

    let state_inner = state.clone();
    let remote_inner = remote.clone();
    let send_handler = state.cpu_pool.spawn_fn(move || {
        let state = state_inner.clone();
        let remote = remote_inner.clone();

        // send_channel_in contains the messages that we're queuing up to
        // send to the client at addr.
        send_channel_in.for_each(move |(addr, msg): (String, String)| {
            let state = state.clone();
            let remote = remote.clone();

            info!("Removing {:?} from connections to send", addr);
            let sink_result = match state.connections.try_write() {
                Ok(mut m) => m.remove(&addr),
                Err(_) => {
                    warn!("RwLock has been poisoned!");
                    None
                },
            };

            if let Some(sink) = sink_result {
                info!("Sending {:?} to {:?}", msg, addr);
                let f = sink.send(WsOwnedMessage::Text(msg))
                            .and_then(move |sink| {
                                info!("Re-adding {:?} to connections after send", addr);
                                match state.connections.try_write() {
                                    Ok(mut m) => {
                                        m.insert(addr, sink);
                                        ()
                                    },
                                    Err(_) => warn!("RwLock has been poisoned!"),
                                }
                                Ok(())
                            });

                remote.spawn(move |handle| {
                    spawn_future(f, "Send message on websocket connection", handle);
                    Ok(())
                });
            } else {
                warn!("Unable to find client addr in connections map: {:?}", addr);
            }

            Ok(())
        })
    });

    let state_inner = state.clone();
    let remote_inner = remote.clone();
    // Start watching for messages in subscribed topics.
    let consumer_handler = consumer.start()
        .filter_map(|result| {
            // Discard any errors from the Kafka subscription.
            match result {
                Ok(v) => Some(v),
                Err(e) => {
                    error!("Kafka error: {:?}", e);
                    None
                }
            }
        }).for_each(move |msg| {
            let state = state_inner.clone();
            let remote = remote_inner.clone();
            let owned_message = msg.detach();

            let state = state.clone();
            let remote = remote.clone();

            let parsed_message = parse_consumer_payload(owned_message);
            match parsed_message {
                Ok(m) => send_event_to_clients(&state, remote, m),
                Err(e) => warn!("Unable to parse message from Kafka: {:?}", e),
            }

            Ok(())
        });

    let handlers = connection_handler.select2(receive_handler.select2(
            send_handler.select2(consumer_handler)));
    core.run(handlers)
        .map(|_| ())
        .map_err(|_| ErrorKind::EventLoopRunFailure)
        .map_err(From::from)
}

fn send_event_to_clients(state: &EventLoopState, remote: Remote, message: Event) {
    match state.connections.try_read() {
        Ok(map) => {
            // Loop over the clients.
            for (addr, _) in map.iter() {
                let state = state.clone();
                let parsed_message = message.clone();

                info!("Processing consumer message to {:?}", addr);
                let processed_message = consumer::process_event(parsed_message, addr);

                match processed_message {
                    Ok(m) => {
                        let f = state.send_channel_out.send((addr.to_string(), m));

                        remote.spawn(move |handle| {
                            spawn_future(f, "Send message to write handler", handle);
                            Ok(())
                        });
                    },
                    Err(e) => warn!("Unable to process message from Kafka: {:?}", e),
                }
            }
        },
        Err(_) => warn!("RwLock has been poisoned!"),
    };
}

fn parse_consumer_payload(owned_message: KafkaOwnedMessage) -> Result<Event> {
    info!("Retrieving consumer payload");
    let payload = owned_message.payload().ok_or(ErrorKind::KafkaMessageWithNoPayload)?;

    info!("Converting consumer payload to string");
    let msg = from_utf8(payload).context(ErrorKind::InvalidUtf8Bytes)?;

    info!("Parsing consumer payload as message");
    consumer::parse_message(msg.to_string())
}

fn spawn_future<F, I, E>(f: F, desc: &'static str, handle: &Handle)
    where F: Future<Item = I, Error = E> + 'static, E: Debug
{
    handle.spawn(f.map_err(move |e| error!("Error in {}: '{:?}'", desc, e))
                 .map(move |_| info!("{}: Finished.", desc)));
}
