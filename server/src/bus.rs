use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;

use actix::{Actor, Address, Context};
use common::schemas::{ConsistencyKey, ConsistencyValue, Event};
use couchbase::{Bucket, BinaryDocument};
use failure::{Error, ResultExt};
use rdkafka::client::EmptyContext;
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;
use futures::Future;
use serde_json::{from_str};

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
    /// This field contains the type of this client, it is `None` if the client is not registered.
    pub client_type: Option<String>,
    /// This field contains the address of the session actor that signals should be sent to.
    pub address: Address<Session>,
    /// This field contains the registered types for this client.
    pub registered_types: RegisteredTypes,
    /// This field contains which consistency keys this session is handling through sticky
    /// round robin.
    pub consistency_keys: HashSet<(String, ConsistencyKey)>,
    /// This field contains the unacknowledged messages sent to this session that should be resent
    /// if this session disconnects and fails to acknowledge the finished processing of this event.
    ///
    /// We store the events as strings here since we are unable to implement `Hash` on the `Value`
    /// type from `serde_json`. This should not contain the `message_type` field and should not be
    /// pretty printed.
    pub unacknowledged_events: HashSet<Event>,
}

/// Bus maintains the state that pertains to all clients and allows clients to send messages
/// to each other.
/// Handlers for different types of messages that the bus can handle are implemented in the
/// messages module.
pub struct Bus {
    /// This field contains the majority of the information regarding an individual session. When
    /// a client connects, the `SocketAddr` identifies the client connection and the
    /// `SessionDetails` contains other information required for that session including the event
    /// types it is registered for, if it is registered, the address of the actor and the type of
    /// client.
    pub sessions: HashMap<SocketAddr, SessionDetails>,
    /// This field contains a mapping from the client type to a queue containing the `SocketAddr`
    /// for connected clients that are of that client type. This is useful as we want to be able
    /// to iterate over each type of client and then send to the sessions of that type. Each
    /// `SocketAddr` should be in the `sessions` map.
    pub round_robin_state: HashMap<String, VecDeque<SocketAddr>>,
    /// This field contains a mapping from each sequence key to the `SocketAddr` of the client
    /// that handles the events for that key. It is checked before the round robin state.
    pub sticky_consistency: HashMap<(String, ConsistencyKey), SocketAddr>,
    /// This field contains all messages that are not yet sent out to a client type and should be.
    pub pending_events: HashMap<String, Vec<Event>>,
    /// This field contains the topic that events should be sent to in Kafka.
    pub topic: String,
    /// This field contains the mapping of the sequence key to the last seen sequence value.
    pub consistency: HashMap<ConsistencyKey, ConsistencyValue>,
    /// This field contains the producer that will be used when sending messages to Kafka.
    pub producer: FutureProducer<EmptyContext>,
    /// This field contains the couchbase bucket that will be used when persisting events to
    /// Couchbase.
    pub event_bucket: Bucket,
    /// This field contains the couchbase bucket that will be used when persisting the consistency
    /// map to couchbase.
    pub consistency_bucket: Bucket
}

impl Bus {
    pub fn launch(brokers: &str, topic: &str, couchbase_host: &str) -> Result<Address<Self>, Error> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("produce.offset.report", "true")
            .create::<FutureProducer<_>>()
            .context(ErrorKind::KafkaProducerCreation)?;

        let event_bucket = connect_to_bucket(couchbase_host, "events")?;
        let consistency_bucket = connect_to_bucket(couchbase_host, "consistency")?;

        let consistency = match consistency_bucket.get::<BinaryDocument, _>("consistency").wait() {
            Ok(doc) => {
                let content = doc.content_as_str()?;
                match content {
                    Some(text) => {
                        info!("found existing hashmap in consistency bucket, using that");
                        let map: HashMap<ConsistencyKey, ConsistencyValue> = from_str(text)?;
                        map
                    },
                    None => {
                        info!("empty consistency map found in couchbase, creating new map");
                        HashMap::new()
                    },
                }
            },
            Err(e) => {
                info!("hashmap does not exist, creating new map: error='{:?}'", e);
                HashMap::new()
            },
        };

        Ok(Self {
            sessions: HashMap::new(),
            round_robin_state: HashMap::new(),
            sticky_consistency: HashMap::new(),
            pending_events: HashMap::new(),
            topic: topic.to_owned(),
            consistency: consistency,
            producer: producer,
            event_bucket: event_bucket,
            consistency_bucket: consistency_bucket,
        }.start())
    }
}

impl Actor for Bus {
    type Context = Context<Self>;
}
