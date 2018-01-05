use std::collections::HashMap;
use std::net::SocketAddr;

use actix::{Actor, Address, Context};
use failure::{ResultExt, Error};
use rdkafka::client::EmptyContext;
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;

use error::ErrorKind;
use session::Session;

/// Bus maintains the state that pertains to all clients and allows clients to send messages
/// to each other.
///
/// Handlers for different types of messages that the bus can handle are implemented in the
/// messages module.
pub struct Bus {
    pub sessions: HashMap<SocketAddr, Address<Session>>,
    pub topic: String,
    pub producer: FutureProducer<EmptyContext>,
}

impl Bus {
    pub fn launch(brokers: &str, topic: &str) -> Result<Address<Self>, Error> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("produce.offset.report", "true")
            .create::<FutureProducer<_>>()
            .context(ErrorKind::KafkaProducerCreation)?;

        Ok(Self {
            sessions: HashMap::new(),
            topic: topic.to_owned(),
            producer: producer,
        }.start())
    }
}

impl Actor for Bus {
    type Context = Context<Self>;
}

