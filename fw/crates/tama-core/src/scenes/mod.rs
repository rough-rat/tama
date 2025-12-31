use embedded_graphics::prelude::DrawTarget;
use enum_dispatch::enum_dispatch;

use crate::{consts, engine::{Context, DrawContext}, scenes::{dvd::DvdScene, flappy::FlappyScene, menu::MenuScene, selftest::SelfTestScene, ui_test::UiTestScene}};

pub mod dvd;
pub mod flappy;
pub mod menu;
pub mod selftest;
pub mod ui_test;

pub enum UpdateResult {
    None,
    // will have more than one scene type
    ChangeScene(SceneWrapper),
}

#[enum_dispatch]
pub trait Scene {
    fn update(&mut self, ctx: &mut Context) -> UpdateResult;
    fn draw<D>(&self, target: &mut D, ctx: &DrawContext) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>;
}

// need a better name
#[enum_dispatch(Scene)]
pub enum SceneWrapper {
    MenuScene,
    FlappyScene,
    SelfTestScene,
    UiTestScene,
    DvdScene,
}
