use embedded_graphics::prelude::DrawTarget;

use crate::consts;

pub mod dvd;

pub trait Scene {
    fn update(&mut self);
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>;
}
