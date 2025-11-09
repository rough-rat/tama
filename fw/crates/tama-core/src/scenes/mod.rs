use embedded_graphics::prelude::DrawTarget;
use enum_dispatch::enum_dispatch;

use crate::{consts, engine::Context, scenes::{flappy::FlappyScene, menu::MenuScene, selftest::SelfTestScene}};

pub mod dvd;
pub mod flappy;
pub mod menu;
pub mod selftest;

pub enum UpdateResult {
    None,
    // will have more than one scene type
    ChangeScene(SceneWrapper),
}

#[enum_dispatch]
pub trait Scene {
    fn update(&mut self, ctx: &mut Context) -> UpdateResult;
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>;
}

// need a better name
#[enum_dispatch(Scene)]
pub enum SceneWrapper {
    MenuScene,
    FlappyScene,
    SelfTestScene,
}
