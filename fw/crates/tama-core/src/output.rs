use crate::buzzer::BuzzerTrait;

pub struct Output<'a> {
    buzzer: &'a dyn BuzzerTrait,
}

impl<'a> Output<'a> {
    pub fn new(buzzer: &'a dyn BuzzerTrait) -> Self {
        Self { buzzer }
    }

    pub fn play_tone(&self, frequency_hz: u32, duration_ms: u32) {
        self.buzzer.beep(frequency_hz, duration_ms);
    }
}
