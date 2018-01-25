use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;

use actix::{Actor, Address, Context};
use couchbase::{Bucket};
use failure::{Error, ResultExt};
use rdkafka::client::EmptyContext;
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;

use error::ErrorKind;
use persistence::connect_to_bucket;
use session::Session;

/// RegisteredTypes represents which types of events a given client is interested in,
/// all events or a subset of events.
#[derive(Clone, Debug)]
pub enum RegisteredTypes {
    All,
    Some(Vec<String>),
}

/// SessionDetails contains all the information that relates to a given session that is
/// connected.
#[derive(Clone)]
pub struct SessionDetails {
    pub client_type: Option<String>,
    pub address: Address<Session>,
    pub registered_types: RegisteredTypes,
    pub is_registered: bool,
}
pub type SequenceKey = String;
pub type SequenceValue = u32;

/// Bus maintains the state that pertains to all clients and allows clients to send messages
/// to each other.
/// Handlers for different types of messages that the bus can handle are implemented in the
/// messages module.
pub struct Bus {
    pub sessions: HashMap<SocketAddr, SessionDetails>,
    pub round_robin_state: HashMap<String, VecDeque<SocketAddr>>,
    pub topic: String,
    pub consistency: HashMap<SequenceKey, SequenceValue>,
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
            round_robin_state: HashMap::new(),
            topic: topic.to_owned(),
            consistency: HashMap::new(),
            producer: producer,
            couchbase_bucket: bucket,
        }.start())
    }
}

impl Actor for Bus {
    type Context = Context<Self>;
}
