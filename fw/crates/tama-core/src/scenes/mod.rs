use embedded_graphics::prelude::DrawTarget;

use crate::{consts, engine::Context, scenes::flappy::FlappyScene};

pub mod dvd;
pub mod flappy;

pub enum UpdateResult {
    None,
    // will have more than one scene type
    ChangeScene(FlappyScene),
}

pub trait Scene {
    fn update(&mut self, ctx: &mut Context) -> UpdateResult;
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>;
}
