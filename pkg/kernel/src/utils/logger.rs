use log::{Metadata, Record};
use core::fmt::Write;

pub fn parse_log_level(level: &str) -> log::LevelFilter {
    match level {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    }
}

pub fn init(log_level_str: &str) {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();

    // FIXME: Configure the logger
    // log::set_max_level(log::LevelFilter::Trace);

    let max_level = parse_log_level(log_level_str);

    log::set_max_level(max_level);

    info!("Logger Initialized.");
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            // Set the color according to the log level
            let (level_color, level_str) = match record.level() {
                log::Level::Error => ("\x1B[31m", "ERROR"),
                log::Level::Warn  => ("\x1B[33m", "WARN"),
                log::Level::Info  => ("\x1B[32m", "INFO"),
                log::Level::Debug => ("\x1B[34m", "DEBUG"),
                log::Level::Trace => ("\x1B[35m", "TRACE"),
            };
            // ANSI color code reset
            let reset = "\x1B[0m";
            // Get log source file
            let file = record.file_static().unwrap_or("unknown file");

            // Formatted log output
            println!("{}[{}]: {} (from {}){}", level_color, level_str, record.args(), file, reset);
        }
    }

    fn flush(&self) {}
}
