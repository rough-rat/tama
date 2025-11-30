//! Log Buffer - Ring buffer for capturing log messages
//!
//! This module provides a no_std compatible ring buffer for storing
//! log messages that can be displayed on screen for debugging.

use heapless::Deque;

/// Maximum number of log lines to store
pub const LOG_BUFFER_CAPACITY: usize = 32;

/// Maximum length of a single log line
pub const LOG_LINE_MAX_LEN: usize = 80;

/// Log level for filtering
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Notice = 5,  // Above error - for important messages that should always be captured
}

impl LogLevel {
    /// Get a short prefix for the log level
    pub fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Trace => "T",
            LogLevel::Debug => "D",
            LogLevel::Info => "I",
            LogLevel::Warn => "W",
            LogLevel::Error => "E",
            LogLevel::Notice => "N",
        }
    }
}

/// A single log entry
#[derive(Clone, Debug)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,
    /// Log message (truncated to LOG_LINE_MAX_LEN)
    pub message: heapless::String<LOG_LINE_MAX_LEN>,
}

impl LogEntry {
    /// Create a new log entry, truncating message if needed
    pub fn new(level: LogLevel, message: &str) -> Self {
        let mut msg = heapless::String::new();
        // Truncate to fit
        for c in message.chars().take(LOG_LINE_MAX_LEN - 1) {
            if msg.push(c).is_err() {
                break;
            }
        }
        Self { level, message: msg }
    }
}

/// Ring buffer for log entries
/// 
/// Thread-safe when wrapped in appropriate synchronization primitive.
/// Uses a fixed-size deque to store recent log entries.
pub struct LogBuffer {
    entries: Deque<LogEntry, LOG_BUFFER_CAPACITY>,
    /// Minimum level to capture (entries below this are ignored)
    min_level: LogLevel,
    /// Whether capture is enabled
    enabled: bool,
}

impl LogBuffer {
    /// Create a new empty log buffer
    pub const fn new() -> Self {
        Self {
            entries: Deque::new(),
            min_level: LogLevel::Info,
            enabled: true,
        }
    }
    
    /// Set the minimum log level to capture
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }
    
    /// Get the minimum log level
    pub fn min_level(&self) -> LogLevel {
        self.min_level
    }
    
    /// Enable or disable log capture
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Check if capture is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Push a log entry, removing oldest if full
    pub fn push(&mut self, level: LogLevel, message: &str) {
        if !self.enabled || level < self.min_level {
            return;
        }
        
        // If full, remove oldest entry
        if self.entries.is_full() {
            self.entries.pop_front();
        }
        
        let entry = LogEntry::new(level, message);
        // This should never fail since we just made room
        let _ = self.entries.push_back(entry);
    }
    
    /// Push a pre-formatted log entry
    pub fn push_entry(&mut self, entry: LogEntry) {
        if !self.enabled || entry.level < self.min_level {
            return;
        }
        
        if self.entries.is_full() {
            self.entries.pop_front();
        }
        let _ = self.entries.push_back(entry);
    }
    
    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    /// Iterate over entries (oldest first)
    pub fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }
    
    /// Get the most recent N entries (newest first)
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter().rev().take(count)
    }
    
    /// Get entry by index (0 = oldest)
    pub fn get(&self, index: usize) -> Option<&LogEntry> {
        self.entries.get(index)
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_push_and_iterate() {
        let mut buffer = LogBuffer::new();
        buffer.push(LogLevel::Info, "First message");
        buffer.push(LogLevel::Warn, "Second message");
        
        assert_eq!(buffer.len(), 2);
        
        let entries: Vec<_> = buffer.iter().collect();
        assert_eq!(entries[0].message.as_str(), "First message");
        assert_eq!(entries[1].message.as_str(), "Second message");
    }
    
    #[test]
    fn test_level_filtering() {
        let mut buffer = LogBuffer::new();
        buffer.set_min_level(LogLevel::Warn);
        
        buffer.push(LogLevel::Info, "Should be ignored");
        buffer.push(LogLevel::Warn, "Should be captured");
        
        assert_eq!(buffer.len(), 1);
    }
    
    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer = LogBuffer::new();
        
        // Fill beyond capacity
        for i in 0..(LOG_BUFFER_CAPACITY + 5) {
            buffer.push(LogLevel::Info, &format!("Message {}", i));
        }
        
        // Should have exactly capacity entries
        assert_eq!(buffer.len(), LOG_BUFFER_CAPACITY);
        
        // Oldest should be message 5 (0-4 were pushed out)
        assert!(buffer.get(0).unwrap().message.starts_with("Message 5"));
    }
}
