use slog;
use slog_scope;
use slog_stdlog;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;

pub fn default_logger() -> (slog::Logger, slog_scope::GlobalLoggerGuard) {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    let drain = builder.build().unwrap();
    let scope_guard = slog_scope::set_global_logger(drain.clone());
    slog_stdlog::init().unwrap();
    (drain, scope_guard)
}

pub fn test_logger() -> slog::Logger {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    builder.build().unwrap()
}
