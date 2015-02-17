//! Minimal logging struct.

use log::{Log, LogLevel, LogRecord};

/// Minimal logging struct.
pub struct Logger;

impl Log for Logger {
  fn enabled(&self, _: LogLevel, _: &str) -> bool {
    true
  }

  fn log(&self, record: &LogRecord) {
    println!("{}:{}: {}", record.level(), record.location().module_path, record.args());
  }
}
