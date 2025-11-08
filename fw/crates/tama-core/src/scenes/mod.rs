use embedded_graphics::prelude::DrawTarget;

use crate::{consts, engine::Context};

pub mod dvd;
pub mod flappy;

pub trait Scene {
    fn update(&mut self, ctx: &mut Context);
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>;
}
