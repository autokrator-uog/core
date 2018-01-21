use std::collections::HashMap;
use std::net::SocketAddr;

use actix::{Actor, Address, Context};
use failure::{Error, ResultExt};
use rdkafka::client::EmptyContext;
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;

use couchbase::{Bucket};
use persistence::connect_to_bucket;

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
    pub couchbase_bucket: Bucket
}

impl Bus {
    pub fn launch(brokers: &str, topic: &str, couchbase_host: &str) -> Result<Address<Self>, Error> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("produce.offset.report", "true")
            .create::<FutureProducer<_>>()
            .context(ErrorKind::KafkaProducerCreation)?;
        
        let bucket = connect_to_bucket(couchbase_host)?;

        Ok(Self {
            sessions: HashMap::new(),
            topic: topic.to_owned(),
            producer: producer,
            couchbase_bucket: bucket,
        }.start())
    }
}

impl Actor for Bus {
    type Context = Context<Self>;
}
