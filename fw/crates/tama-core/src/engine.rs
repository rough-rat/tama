
use embedded_graphics::{
    prelude::DrawTarget,
};

use crate::{consts, scenes::{Scene as _, dvd::DvdScene}};



pub struct Engine {
    scene: DvdScene,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            scene: DvdScene::new(),
        }
    }

    pub fn render<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        self.scene.draw(target)
    }

    pub fn update(&mut self) {
        self.scene.update();
    }
}
