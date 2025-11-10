
use embedded_graphics::{
    prelude::DrawTarget,
};
use rand::{SeedableRng, rngs::SmallRng};

use crate::{buzzer::BuzzerTrait, consts, input::Input, output::Output, scenes::{Scene as _, SceneWrapper, UpdateResult, selftest::SelfTestScene}};

// Default stub buzzer for embedded targets
pub struct StubBuzzer;

impl BuzzerTrait for StubBuzzer {
    fn beep(&self, _frequency_hz: u32, _duration_ms: u32) {
        // Stub implementation for buzzer
    }
}

pub struct Engine<B: BuzzerTrait = StubBuzzer> {
    scene: SceneWrapper,
    buzzer: B,
    rng: SmallRng,
    input: Input,
}

impl Default for Engine<StubBuzzer> {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine<StubBuzzer> {
    pub fn new() -> Self {
        Self::with_buzzer(StubBuzzer)
    }
}

impl<B: BuzzerTrait> Engine<B> {
    pub fn with_buzzer(buzzer: B) -> Self {
        Self {
            scene: SceneWrapper::from(SelfTestScene::new()),
            buzzer,
            rng: SmallRng::seed_from_u64(2137),
            input: Input::new(),
        }
    }

    pub fn render<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        self.scene.draw(target)
    }

    pub fn update(&mut self) {
        // Create Context on the fly with references to buzzer
        let mut context = Context::new(&self.buzzer);
        // Temporarily swap input to avoid borrowing issues
        core::mem::swap(&mut context.input, &mut self.input);
        core::mem::swap(&mut context.rng, &mut self.rng);
        
        let result = self.scene.update(&mut context);
        
        // Swap back
        core::mem::swap(&mut context.input, &mut self.input);
        core::mem::swap(&mut context.rng, &mut self.rng);

        match result {
            UpdateResult::ChangeScene(scene) => self.scene = scene,
            UpdateResult::None => (),
        }
    }

    pub fn play_tone(&self, frequency_hz: u32, duration_ms: u32) {
        self.buzzer.beep(frequency_hz, duration_ms);
    }

    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input
    }
}

pub struct Context<'a> {
    pub rng: SmallRng,
    pub input: Input,
    pub output: Output<'a>,
}

impl<'a> Context<'a> {
    fn new(buzzer: &'a dyn BuzzerTrait) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(2137),
            input: Input::new(),
            output: Output::new(buzzer),
        }
    }
}

