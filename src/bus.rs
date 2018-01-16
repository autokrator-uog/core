
use std::collections::HashMap;
use std::net::SocketAddr;

use actix::{Actor, Address, Context};
use failure::{ResultExt, Error};
use rdkafka::client::EmptyContext;
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;

// extern crate couchbase;
use couchbase::{Cluster, Bucket};

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
    
    // pub couchbase_cluster: Cluster,
    pub couchbase_bucket: Bucket
}

impl Bus {
    pub fn launch(brokers: &str, topic: &str, couchbase_host: &str) -> Result<Address<Self>, Error> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("produce.offset.report", "true")
            .create::<FutureProducer<_>>()
            .context(ErrorKind::KafkaProducerCreation)?;
            
        let mut cluster = Cluster::new(couchbase_host).expect("Error connecting to cluster");
        cluster.authenticate("connect", "connect");
        
        let bucket = cluster.open_bucket("events", None).expect("Error opening events bucket");

        Ok(Self {
            sessions: HashMap::new(),
            topic: topic.to_owned(),
            producer: producer,
            // couchbase_cluster: cluster,
            couchbase_bucket: bucket,
        }.start())
    }
}

impl Actor for Bus {
    type Context = Context<Self>;
}
