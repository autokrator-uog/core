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
    #[fail(display = "Invalid websocket connection accepted")]
    InvalidWebsocketConnection,

    #[fail(display = "Missing websocket server argument. This is a bug, should be a default")]
    MissingWebsocketServerArgument,
    #[fail(display = "Missing Lua script argument. This is a bug, should be a default")]
    MissingLuaScriptArgument,

    #[fail(display = "Could not find a file at the provided Lua script path")]
    LuaScriptNotFound,
    #[fail(display = "Could not read a Lua script at the provided file path")]
    ReadLuaScript,
    #[fail(display = "Could not evaluate Lua script")]
    EvaluateLuaScript,
    #[fail(display = "Unable to create register function")]
    CreateRegisterFunction,
    #[fail(display = "Could not inject register function into Lua globals")]
    InjectRegisterFunction,
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
