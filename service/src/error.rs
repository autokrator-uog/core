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
    #[fail(display = "Unable to create client builder")]
    WebsocketClientBuilderCreate,
    #[fail(display = "Received invalid message type over websockets")]
    InvalidWebsocketMessageType,
    #[fail(display = "Invalid JSON received on websockets")]
    ParseJsonFromWebsockets,
    #[fail(display = "No message type in JSON from websockets")]
    NoMessageTypeFromWebsockets,

    #[fail(display = "Unable to bind to http port")]
    HttpBindToPort,

    #[fail(display = "Client has not linked to interpreter")]
    ClientNotLinkedToInterpreter,

    #[fail(display = "Missing websocket server argument. This is a bug, should be a default")]
    MissingWebsocketServerArgument,
    #[fail(display = "Missing bind address argument. This is a bug, should be a default")]
    MissingBindAddressArgument,
    #[fail(display = "Missing redis address argument. This is a bug, should be a default")]
    MissingRedisAddressArgument,
    #[fail(display = "Missing Lua script argument. This is a bug, should be a default")]
    MissingLuaScriptArgument,

    #[fail(display = "Failed to create Redis client")]
    RedisClientCreate,
    #[fail(display = "Failed to persist key/value to Redis")]
    RedisPersist,
    #[fail(display = "Failed to query key in Redis")]
    RedisQuery,

    #[fail(display = "Could not find Bus userdata in Lua globals")]
    MissingBusUserData,

    #[fail(display = "Could not find a file at the provided Lua script path")]
    LuaScriptNotFound,
    #[fail(display = "Could not read a Lua script at the provided file path")]
    ReadLuaScript,
    #[fail(display = "Could not evaluate Lua script")]
    EvaluateLuaScript,

    #[fail(display = "Event handler not found in Lua register, was register function invoked?")]
    MissingEventHandlerRegistryValue,
    #[fail(display = "HTTP handler not found in Lua register, was register function invoked?")]
    MissingHttpHandlerRegistryValue,

    #[fail(display = "Failure when running event handler")]
    FailedEventHandler,
    #[fail(display = "Failure when running HTTP handler")]
    FailedHttpHandler,
    #[fail(display = "Failure when parsing result from HTTP handler")]
    ParseHttpHandlerResult,

    #[fail(display = "Failed to serialize value to json for sending")]
    SerializeJsonForSending,
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
