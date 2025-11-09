use embedded_graphics::{
    Drawable as _,
    mono_font::{MonoTextStyleBuilder, ascii::FONT_4X6},
    prelude::{DrawTarget, Point, RgbColor},
    text::{Alignment, Text},
};

use crate::{
    consts, input::Button, scenes::{Scene, SceneWrapper, UpdateResult, flappy::FlappyScene}
};

pub struct MenuScene;

impl MenuScene {
    pub fn new() -> Self {
        Self {}
    }
}

impl Scene for MenuScene {
    fn update(&mut self, ctx: &mut crate::engine::Context) -> UpdateResult {
        if ctx.input.is_just_pressed(Button::A) {
            return UpdateResult::ChangeScene(SceneWrapper::from(FlappyScene::new()))
        }
        UpdateResult::None
    }

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        target.clear(consts::ColorType::WHITE)?;

        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_4X6)
            .text_color(consts::ColorType::BLACK)
            .build();

        Text::with_alignment(
            "Press A to start",
            Point::new(consts::WIDTH as i32 / 2, consts::HEIGHT as i32 / 2),
            text_style,
            Alignment::Center,
        )
        .draw(target)?;

        Ok(())
    }
}
