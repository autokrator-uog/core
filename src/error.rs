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
    KafkaConsumerCreationFailure,
    #[fail(display = "Unable to subscribe to Kafka topic")]
    KafkaConsumerSubscriptionFailure,
    #[fail(display = "Received error from Kafka subscription")]
    KafkaErrorReceived,

    #[fail(display = "Failure when creating websocket server")]
    UnableToBindWebsocketServer,
    #[fail(display = "Invalid websocket connection accepted")]
    InvalidWebsocketConnection,

    #[fail(display = "No bind argument was provided. This is a bug, there should be a default.")]
    MissingBindArgument,
    #[fail(display = "No brokers argument was provided. This is a bug, there should be a default.")]
    MissingBrokersArgument,
    #[fail(display = "No group argument was provided. This is a bug, there should be a default.")]
    MissingGroupArgument,
    #[fail(display = "No topic argument was provided. This is a bug, there should be a default.")]
    MissingTopicArgument,

    #[fail(display = "Encoding failure in websocket codec wrapper")]
    WebsocketCodecWrapperEncoding,
    #[fail(display = "Decoding failure in websocket codec wrapper")]
    WebsocketCodecWrapperDecoding,
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
