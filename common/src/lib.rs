extern crate chrono;
extern crate colored;
#[macro_use] extern crate failure;
extern crate fern;
extern crate log;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

mod error;
pub mod schemas;

use colored::*;
use log::{LogLevel, LogLevelFilter};

pub fn configure_logging(level: LogLevelFilter) {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let now = chrono::Local::now();

            let level_colour = match record.level() {
                LogLevel::Debug => "blue",
                LogLevel::Info => "green",
                LogLevel::Warn => "yellow",
                LogLevel::Error => "red",
                _ => "white"
            };
            let level = format!("{:?}", record.level()).to_uppercase().color(level_colour);

            out.finish(format_args!(
                "[{} {}] [{}] {} {}",
                now.format("%Y-%m-%d"),
                now.format("%H:%M:%S"),
                record.target(),
                level,
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply().unwrap();
}
