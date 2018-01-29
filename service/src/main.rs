#[macro_use] extern crate clap;
extern crate log;
extern crate rlua;
extern crate vicarius_common;

use clap::{Arg, App, AppSettings};
use log::LogLevelFilter;
use vicarius_common::configure_logging;

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
        .get_matches();

    let level = value_t!(matches, "log-level", LogLevelFilter).unwrap_or(LogLevelFilter::Trace);
    configure_logging(level);
}
