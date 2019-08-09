use slog;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;

pub fn default_logger() -> slog::Logger {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Info);
    builder.destination(Destination::Stderr);
    builder.build().unwrap()
}
