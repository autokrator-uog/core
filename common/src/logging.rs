use std::io::stdout;

use chrono::Local;
use colored::*;
use fern::Dispatch;
use log::{LogLevel, LogLevelFilter};

pub fn configure_logging(level: LogLevelFilter) {
    Dispatch::new()
        .format(|out, message, record| {
            let now = Local::now();

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
        .chain(stdout())
        .apply().unwrap();
}
