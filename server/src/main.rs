extern crate actix;
extern crate bytes;
extern crate chrono;
#[macro_use] extern crate clap;
extern crate couchbase;
#[macro_use] extern crate failure;
extern crate futures;
#[macro_use] extern crate log;
extern crate rand;
extern crate rdkafka;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate sha1;
extern crate tokio_core;
extern crate tokio_io;
extern crate vicarius_common;
extern crate websocket;

mod bus;
mod consumer;
mod error;
mod helpers;
mod persistence;
mod schemas;
mod server;
mod session;
mod signals;

use actix::{Address, System};
use clap::{Arg, ArgMatches, App, AppSettings, SubCommand};
use failure::Error;
use log::LogLevelFilter;
use vicarius_common::configure_logging;

use bus::Bus;
use consumer::Consumer;
use error::ErrorKind;
use server::Server;

fn main() {
    let matches = App::new(crate_name!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
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
        .subcommand(SubCommand::with_name("server")
                    .about("Start the event bus daemon")
                    .version(crate_version!())
                    .author(crate_authors!())
                    .arg(Arg::with_name("topic")
                         .short("t")
                         .long("topic")
                         .help("Topic to send and receive messages on")
                         .default_value("sed-instance-1")
                         .takes_value(true))
                    .arg(Arg::with_name("bind")
                         .short("b")
                         .long("bind")
                         .help("Host and port to bind websocket server to")
                         .default_value("localhost:8081")
                         .takes_value(true))
                    .arg(Arg::with_name("brokers")
                         .long("broker")
                         .help("Broker list in Kafka format")
                         .default_value("localhost:9092")
                         .takes_value(true))
                    .arg(Arg::with_name("group")
                         .short("g")
                         .long("group")
                         .help("Consumer group name")
                         .default_value(crate_name!())
                         .takes_value(true))
                    .arg(Arg::with_name("couchbase_host")
                        .long("couchbase-host")
                        .help("The hostname for the couchbase DB.")
                        .default_value("couchbase.db")
                        .takes_value(true))
        ).get_matches();

    let level = value_t!(matches, "log-level", LogLevelFilter).unwrap_or(LogLevelFilter::Trace);
    configure_logging(level);

    match matches.subcommand() {
        ("server", Some(arguments)) => {
            if let Err(e) = start_server(&arguments) {
                error!("failed to start server: error='{}'", e);
            }
        },
        _ => { }
    };
}

fn start_server(arguments: &ArgMatches) -> Result<(), Error> {
    let system = System::new("event-bus");

    // Create the event bus actor, we'll pass this to the websocket server actor
    // and the consumer actor so that they can send it things.
    let brokers = arguments.value_of("brokers").ok_or(ErrorKind::MissingBrokersArgument)?;
    let topic = arguments.value_of("topic").ok_or(ErrorKind::MissingTopicArgument)?;
    let couchbase_host = arguments.value_of("couchbase_host").ok_or(ErrorKind::MissingCouchbaseHostArgument)?;

    let bus: Address<_> = Bus::launch(brokers, topic, couchbase_host)?;

    // Start WebSocket server.
    let addr = arguments.value_of("bind").ok_or(ErrorKind::MissingBindArgument)?;
    Server::launch(addr, bus.clone())?;

    let group = arguments.value_of("group").ok_or(ErrorKind::MissingGroupArgument)?;
    Consumer::launch(brokers, group, topic, bus.clone())?;

    system.run();
    Ok(())
}

