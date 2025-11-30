#![no_std]

extern crate alloc;

pub mod buzzer;
pub mod consts;
pub mod engine;
pub mod input;
pub mod log_buffer;
pub mod output;

mod scenes;

/// Log a notice-level message (above error, always captured).
/// 
/// This uses log::error! with a special target "NOTICE" that the
/// log capture system recognizes and treats as LogLevel::Notice.
#[macro_export]
macro_rules! notice {
    ($($arg:tt)*) => {
        log::error!(target: "NOTICE", $($arg)*)
    };
}

