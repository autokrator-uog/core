#![deny(warnings)]
#![deny(missing_debug_implementations)]

extern crate chrono;
extern crate colored;
#[macro_use] extern crate failure;
extern crate fern;
extern crate log;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate sha1;
extern crate websocket;

mod error;
mod extensions;
mod helpers;
mod logging;
pub mod schemas;

pub use extensions::VecDequeExt;
pub use helpers::hash_json;
pub use logging::configure_logging;
