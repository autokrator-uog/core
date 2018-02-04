extern crate actix;
extern crate actix_web;
#[macro_use] extern crate clap;
#[macro_use] extern crate failure;
extern crate http;
#[macro_use] extern crate log;
extern crate serde;
extern crate serde_json;
extern crate redis;
extern crate rlua;
extern crate vicarius_common;
extern crate websocket;

mod client;
mod error;
mod interpreter;
mod signals;
mod web;

use std::process::exit;

use actix::{Arbiter, SyncAddress, System};
use clap::{Arg, ArgMatches, App};
use failure::Error;
use log::LogLevelFilter;
use vicarius_common::configure_logging;

use client::Client;
use error::ErrorKind;
use interpreter::Interpreter;
use web::start_webserver;

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
        .arg(Arg::with_name("bind-address")
             .short("b")
             .long("bind")
             .help("Address for binding HTTP server")
             .default_value("0.0.0.0:8080")
             .takes_value(true))
        .arg(Arg::with_name("redis-address")
             .short("r")
             .long("redis")
             .help("Redis server address")
             .default_value("redis://localhost/")
             .takes_value(true))
        .arg(Arg::with_name("server-address")
             .short("s")
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

    if let Err(e) = start_client(matches) {
        error!("failed to start client: error='{}'", e);
    }
}

fn start_client(arguments: ArgMatches) -> Result<(), Error> {
    let system = System::new(crate_name!());

    let bind_address = arguments.value_of("bind-address").ok_or(
        ErrorKind::MissingBindAddressArgument)?.to_owned();
    let redis_address = arguments.value_of("redis-address").ok_or(
        ErrorKind::MissingRedisAddressArgument)?.to_owned();
    let server_address = arguments.value_of("server-address").ok_or(
        ErrorKind::MissingWebsocketServerArgument)?.to_owned();
    let script_path = arguments.value_of("input").ok_or(
        ErrorKind::MissingLuaScriptArgument)?.to_owned();

    info!("starting websocket client: server='{}'", server_address);
    let interpreter: SyncAddress<_> = Arbiter::start(|_| {
        match Interpreter::new(script_path, redis_address) {
            Ok(interpreter) => interpreter,
            Err(e) => {
                error!("failed to create interpreter: error='{:?}'", e);
                exit(1);
            },
        }
    });

    // Start the webserver, it needs the address of the interpreter.
    start_webserver(bind_address, interpreter.clone())?;

    Client::launch(server_address, interpreter.clone())?;

    system.run();
    Ok(())
}
