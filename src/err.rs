use std::fmt;
use std::fmt::Display;
use std::result::Result as StdResult;
use failure::{Backtrace, Context, Fail};

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

/// Contains all of the different varieties of errors. By using the ErrorKind pattern with the
/// failure library, we are able to have a one-to-many mapping with the underlying error types and
/// the kind of error. These variants should not carry data.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Unable to convert `Vec<u8>` into `String`")]
    InvalidUtf8Bytes,
    #[fail(display = "Unable to parse incoming JSON message")]
    IncomingJsonParse,
    #[fail(display = "Unable to parse JSON from Kafka")]
    KafkaJsonParse,

    #[fail(display = "Unable to parse new event message")]
    NewEventMessageParse,
    #[fail(display = "Unable to parse query message")]
    QueryMessageParse,
    #[fail(display = "Unable to parse register message")]
    RegisterMessageParse,

    #[fail(display = "No message type provided")]
    NoMessageTypeProvided,
    #[fail(display = "Invalid message type")]
    InvalidMessageType,
    #[fail(display = "Invalid websocket message type")]
    InvalidWebsocketMessageType,

    #[fail(display = "Failure while consumer is processing message")]
    ConsumerProcessingFailure,
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
