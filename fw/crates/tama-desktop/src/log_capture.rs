//! Log capture module - captures log messages to a ring buffer for on-screen display.
//!
//! This module implements a custom `log::Log` that captures messages to a shared
//! `LogBuffer` for display on screen, while also printing to console.

use std::sync::Mutex;
use log::{Level, Log, Metadata, Record};
use tama_core::log_buffer::{LogBuffer, LogEntry, LogLevel};

/// Global log buffer for capturing log messages.
static LOG_BUFFER: Mutex<LogBuffer> = Mutex::new(LogBuffer::new());

/// Custom logger that captures to ring buffer and prints to console.
pub struct CaptureLogger {
    max_level: Level,
}

impl CaptureLogger {
    pub const fn new(max_level: Level) -> Self {
        Self { max_level }
    }
}

impl Log for CaptureLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Determine log level - check for NOTICE target first
        let level = if record.target() == "NOTICE" {
            LogLevel::Notice
        } else {
            match record.level() {
                Level::Error => LogLevel::Error,
                Level::Warn => LogLevel::Warn,
                Level::Info => LogLevel::Info,
                Level::Debug => LogLevel::Debug,
                Level::Trace => LogLevel::Trace,
            }
        };

        // Only capture Warn level and above to the ring buffer
        // (Notice is above Error, so it's always captured)
        if level >= LogLevel::Warn {
            if let Ok(mut buffer) = LOG_BUFFER.lock() {
                let msg = format!("{}", record.args());
                buffer.push(level, &msg);
            }
        }

        // Print to console
        let level_str = if record.target() == "NOTICE" {
            "NOTICE"
        } else {
            match record.level() {
                Level::Error => "ERROR",
                Level::Warn => "WARN",
                Level::Info => "INFO",
                Level::Debug => "DEBUG",
                Level::Trace => "TRACE",
            }
        };
        println!("[{}] {}: {}", level_str, record.target(), record.args());
    }

    fn flush(&self) {}
}

/// Global logger instance.
static LOGGER: CaptureLogger = CaptureLogger::new(Level::Info);

/// Initialize the log capture system.
pub fn init(max_level: log::LevelFilter) {
    match log::set_logger(&LOGGER) {
        Ok(()) => {
            log::set_max_level(max_level);
        }
        Err(_) => {
            // Logger already set
        }
    }
}

/// Get a snapshot of recent log entries as LogEntry structs.
pub fn recent_log_entries(count: usize) -> Vec<LogEntry> {
    LOG_BUFFER
        .lock()
        .ok()
        .map(|buffer| buffer.recent(count).cloned().collect())
        .unwrap_or_default()
}
