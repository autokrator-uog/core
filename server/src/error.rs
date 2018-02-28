use std::fmt;
use std::fmt::Display;
use failure::{Backtrace, Context, Fail};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

/// Contains all of the different varieties of errors. By using the ErrorKind pattern with the
/// failure library, we are able to have a one-to-many mapping with the underlying error types and
/// the kind of error. These variants should not carry data.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Unable to create Kafka consumer")]
    KafkaConsumerCreation,
    #[fail(display = "Unable to create Kafka producer")]
    KafkaProducerCreation,
    #[fail(display = "Unable to subscribe to Kafka topic")]
    KafkaConsumerSubscription,
    #[fail(display = "Received error from Kafka subscription")]
    KafkaErrorReceived,

    #[fail(display = "Failure when creating websocket server")]
    UnableToBindWebsocketServer,
    #[fail(display = "Invalid websocket connection accepted")]
    InvalidWebsocketConnection,

    #[fail(display = "No bind argument was provided. This is a bug, there should be a default")]
    MissingBindArgument,
    #[fail(display = "No brokers argument was provided. This is a bug, there should be a default")]
    MissingBrokersArgument,
    #[fail(display = "No group argument was provided. This is a bug, there should be a default")]
    MissingGroupArgument,
    #[fail(display = "No topic argument was provided. This is a bug, there should be a default")]
    MissingTopicArgument,
    #[fail(display = "No couchbase_host argument was provided. This is a bug, there should be a default")]
    MissingCouchbaseHostArgument,

    #[fail(display = "Failed to parse bytes as UTF8 string")]
    ParseBytesAsUtf8,
    #[fail(display = "Received invalid message type over websockets")]
    InvalidWebsocketMessageType,
    #[fail(display = "Invalid JSON received on websockets")]
    ParseJsonFromWebsockets,
    #[fail(display = "Invalid JSON received on Kafka")]
    ParseJsonFromKafka,
    #[fail(display = "No message type in JSON from websockets")]
    NoMessageTypeFromWebsockets,

    #[fail(display = "Failed to serialize value to json for sending")]
    SerializeJsonForSending,

    #[fail(display = "Failed to serialize hashmap for persisting")]
    SerializeHashMapForCouchbase,

    #[fail(display = "Message from Kafka with no payload")]
    KafkaMessageWithNoPayload,

    #[fail(display = "Invalid JSON received in new event message")]
    ParseNewEventMessage,
    #[fail(display = "Invalid JSON received in acknowledgement message")]
    ParseAcknowledgement,

    #[fail(display = "Invalid data received in query message")]
    ParseQueryMessage,

    // couchbase errors
    #[fail(display = "Failed to connect to Couchbase")]
    CouchbaseFailedConnect,
    #[fail(display = "Failed to get result of query")]
    CouchbaseFailedGetQueryResult,
    #[fail(display = "Failed to deserialize result of query")]
    CouchbaseDeserialize,
    #[fail(display = "Failed to create GSI")]
    CouchbaseCreateGSIFailed,
    #[fail(display = "Got a row when we weren't expecting one")]
    CouchbaseUnexpectedResultReturned,

    #[fail(display = "The client was not present in the HashMap")]
    SessionNotInHashMap,
    #[fail(display = "No clients in round robin queue for type")]
    RoundRobinEmptyQueue,
    #[fail(display = "No queue for client type in round robin state")]
    RoundRobinNoQueue,

    #[fail(display = "Attempt to resend unacknowledged events with unregistered clients")]
    UnacknowledgedEventResendWithoutClientType,
}

impl Error {
    #[allow(dead_code)]
    pub fn kind(&self) -> ErrorKind { *self.inner.get_context() }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> { self.inner.cause() }
    fn backtrace(&self) -> Option<&Backtrace> { self.inner.backtrace() }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { Display::fmt(&self.inner, f) }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self { Self { inner: Context::new(kind) } }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Self { Self { inner: inner } }
}
