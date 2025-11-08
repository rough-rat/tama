
use embedded_graphics::{
    prelude::DrawTarget,
};
use rand::{SeedableRng, rngs::SmallRng};

use crate::{consts, input::Input, scenes::{Scene as _, flappy::FlappyScene}};


pub struct Engine {
    scene: FlappyScene,
    context: Context,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            scene: FlappyScene::new(),
            context: Context::new(),
        }
    }

    pub fn render<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        self.scene.draw(target)
    }

    pub fn update(&mut self) {
        self.scene.update(&mut self.context);
    }

    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.context.input
    }
}

pub struct Context {
    pub rng: SmallRng,
    pub input: Input,
}

impl Context {
    fn new() -> Self {
        Self {
            rng: SmallRng::seed_from_u64(2137),
            input: Input::new(),
        }
    }
}

