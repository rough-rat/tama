// Platform-agnostic buzzer trait
pub trait BuzzerTrait: Send {
    fn beep(&self, frequency_hz: u32, duration_ms: u32);
}
