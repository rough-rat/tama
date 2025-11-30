//! Log capture module - captures log messages to a ring buffer for on-screen display.
//!
//! This module implements a custom `log::Log` that chains to the ESP-IDF logger
//! while also capturing messages to a shared `LogBuffer` for display on screen.

use core::sync::atomic::{AtomicBool, Ordering};
use log::{Level, Log, Metadata, Record};
use std::sync::Mutex;
use tama_core::log_buffer::{LogBuffer, LogEntry, LogLevel};

/// Global log buffer for capturing log messages.
static LOG_BUFFER: Mutex<LogBuffer> = Mutex::new(LogBuffer::new());

/// Flag to prevent recursive logging (if logging itself causes a log).
static LOGGING_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Custom logger that captures to ring buffer and chains to ESP logger.
pub struct CaptureLogger {
    /// The original ESP-IDF logger max level.
    esp_max_level: Level,
}

impl CaptureLogger {
    /// Create a new capture logger.
    pub const fn new(esp_max_level: Level) -> Self {
        Self { esp_max_level }
    }
}

impl Log for CaptureLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.esp_max_level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Prevent recursive logging
        if LOGGING_IN_PROGRESS.swap(true, Ordering::SeqCst) {
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
                use std::fmt::Write;
                let mut msg = String::new();
                let _ = write!(msg, "{}", record.args());
                buffer.push(level, &msg);
            }
        }

        // Chain to ESP-IDF logging via esp_log_write
        // This bypasses the log crate and goes directly to ESP-IDF
        unsafe {
            let level = match record.level() {
                Level::Error => esp_idf_svc::sys::esp_log_level_t_ESP_LOG_ERROR,
                Level::Warn => esp_idf_svc::sys::esp_log_level_t_ESP_LOG_WARN,
                Level::Info => esp_idf_svc::sys::esp_log_level_t_ESP_LOG_INFO,
                Level::Debug => esp_idf_svc::sys::esp_log_level_t_ESP_LOG_DEBUG,
                Level::Trace => esp_idf_svc::sys::esp_log_level_t_ESP_LOG_VERBOSE,
            };
            
            // Format target and message as C strings
            let target = std::ffi::CString::new(record.target()).unwrap_or_default();
            let message = std::ffi::CString::new(format!("{}", record.args())).unwrap_or_default();
            
            esp_idf_svc::sys::esp_log_write(
                level,
                target.as_ptr() as *const u8,
                b"%s\n\0".as_ptr(),
                message.as_ptr(),
            );
        }

        LOGGING_IN_PROGRESS.store(false, Ordering::SeqCst);
    }

    fn flush(&self) {
        // Nothing to flush for the ring buffer
    }
}

/// Global logger instance.
static LOGGER: CaptureLogger = CaptureLogger::new(Level::Info);

/// Initialize the log capture system.
///
/// This should be called early in main(), before any other initialization
/// that might produce log messages you want to capture.
///
/// # Arguments
/// * `max_level` - Maximum log level to capture and display.
///
/// # Example
/// ```ignore
/// log_capture::init(log::LevelFilter::Info);
/// log::info!("This will be captured!");
/// ```
pub fn init(max_level: log::LevelFilter) {
    // Set our logger as the global logger BEFORE ESP logger tries to set itself
    // Note: set_logger will fail if called more than once, so we do this first
    match log::set_logger(&LOGGER) {
        Ok(()) => {
            log::set_max_level(max_level);
        }
        Err(_) => {
            // Logger already set - this shouldn't happen if we're called first
            // but if it does, we can't capture logs
        }
    }
}

/// Access the log buffer to read captured entries.
///
/// Returns a guard that can be used to iterate over log entries.
pub fn with_buffer<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&LogBuffer) -> R,
{
    LOG_BUFFER.lock().ok().map(|guard| f(&guard))
}

/// Access the log buffer mutably (e.g., to clear it or change settings).
pub fn with_buffer_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut LogBuffer) -> R,
{
    LOG_BUFFER.lock().ok().map(|mut guard| f(&mut guard))
}

/// Get a snapshot of recent log entries.
///
/// Returns a vector of (level, message) tuples for the most recent entries.
/// This copies the data so it can be used without holding the lock.
pub fn recent_entries(count: usize) -> Vec<(LogLevel, String)> {
    with_buffer(|buffer| {
        buffer
            .recent(count)
            .map(|entry| (entry.level, entry.message.to_string()))
            .collect()
    })
    .unwrap_or_default()
}

/// Get a snapshot of recent log entries as LogEntry structs.
///
/// This is the preferred method for passing to the Engine.
pub fn recent_log_entries(count: usize) -> Vec<LogEntry> {
    with_buffer(|buffer| {
        buffer
            .recent(count)
            .cloned()
            .collect()
    })
    .unwrap_or_default()
}

/// Clear all captured log entries.
pub fn clear() {
    with_buffer_mut(|buffer| buffer.clear());
}

/// Set the minimum log level for the buffer.
///
/// Messages below this level won't be stored in the buffer.
pub fn set_min_level(level: LogLevel) {
    with_buffer_mut(|buffer| buffer.set_min_level(level));
}

/// Enable or disable log capture to the buffer.
///
/// When disabled, messages still go to the ESP logger but aren't captured.
pub fn set_enabled(enabled: bool) {
    with_buffer_mut(|buffer| buffer.set_enabled(enabled));
}
