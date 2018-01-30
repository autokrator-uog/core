extern crate actix;
#[macro_use] extern crate clap;
#[macro_use] extern crate failure;
#[macro_use] extern crate log;
extern crate serde;
extern crate serde_json;
extern crate rlua;
extern crate vicarius_common;
extern crate websocket;

mod client;
mod error;
mod interpreter;
mod signals;

use actix::{Address, System};
use clap::{Arg, ArgMatches, App};
use failure::Error;
use log::LogLevelFilter;
use vicarius_common::configure_logging;

use client::Client;
use error::ErrorKind;
use interpreter::Interpreter;

fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::with_name("log-level")
             .short("l")
             .long("log-level")
             .help("Log level")
             .default_value("info")
             .possible_values(&["off", "trace", "debug", "info", "warn", "error"])
             .takes_value(true))
        .arg(Arg::with_name("server-address")
             .long("server")
             .help("Websocket server address")
             .default_value("ws://localhost:8081")
             .takes_value(true))
        .arg(Arg::with_name("input")
             .help("Path to lua script to run as a service")
             .index(1)
             .required(true))
        .get_matches();

    let level = value_t!(matches, "log-level", LogLevelFilter).unwrap_or(LogLevelFilter::Trace);
    configure_logging(level);

    if let Err(e) = start_client(&matches) {
        error!("failed to start client: error='{}'", e);
    }
}

fn start_client(arguments: &ArgMatches) -> Result<(), Error> {
    let system = System::new(crate_name!());

    let script_path = arguments.value_of("input").ok_or(
        ErrorKind::MissingLuaScriptArgument)?;
    let interpreter: Address<_> = Interpreter::launch(script_path)?;

    let server_address = arguments.value_of("server-address").ok_or(
        ErrorKind::MissingWebsocketServerArgument)?;
    Client::launch(server_address, interpreter.clone())?;

    system.run();
    Ok(())
}
