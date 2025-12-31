use core::fmt::Write as _;
use embedded_graphics::{
    prelude::{Dimensions, DrawTarget, Point, RgbColor, Size},
    primitives::Rectangle,
};
use heapless::String;

use crate::{
    consts,
    engine::{Context, DrawContext},
    input::Button,
    scenes::{Scene, SceneWrapper, UpdateResult, dvd::DvdScene, flappy::FlappyScene},
    ui::{draw_button, draw_para, draw_top_bar},
};

const BUTTON_LABELS: [&str; 5] = ["Foo", "Bar", "Baz", "Flappy", "DVD"];

pub struct UiTestScene {
    hot_idx: u32,
    active_idx: Option<u32>,
    buffer: String<128>,
}

impl UiTestScene {
    pub fn new() -> Self {
        Self {
            hot_idx: 0,
            active_idx: None,
            buffer: String::new(),
        }
    }
}

impl Scene for UiTestScene {
    fn update(&mut self, ctx: &mut Context) -> UpdateResult {
        if ctx.input.is_just_pressed(Button::Down) {
            self.hot_idx = (self.hot_idx + 1) % BUTTON_LABELS.len() as u32;
        }

        if ctx.input.is_just_pressed(Button::Up) {
            self.hot_idx = if self.hot_idx == 0 {
                BUTTON_LABELS.len() as u32 - 1
            } else {
                self.hot_idx - 1
            };
        }

        if ctx.input.is_just_pressed(Button::A) {
            self.active_idx = Some(self.hot_idx);
        }

        if ctx.input.is_just_released(Button::A) {
            if self.active_idx == Some(self.hot_idx) {
                match self.hot_idx {
                    0..=2 => {
                        let text = BUTTON_LABELS[self.hot_idx as usize];
                        // fail silently if buffer is exhausted
                        let _ = write!(self.buffer, " {text}");
                    }
                    3 => {
                        self.active_idx = None;
                        return UpdateResult::ChangeScene(SceneWrapper::from(FlappyScene::new()));
                    }
                    4 => {
                        self.active_idx = None;
                        return UpdateResult::ChangeScene(SceneWrapper::from(DvdScene::new()));
                    }
                    _ => {}
                }
            }

            self.active_idx = None;
        }

        UpdateResult::None
    }

    fn draw<D>(&self, target: &mut D, ctx: &DrawContext) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        let size = target.bounding_box().size;

        target.clear(consts::ColorType::WHITE)?;
        draw_top_bar(target, ctx)?;
        let button_size = Size::new(64, 24);
        let buttons_start_y = 64;
        let vertical_margin = 8;

        let mut layout = VerticalLayout::new(
            Point::new(
                size.width as i32 / 2 - button_size.width as i32 / 2,
                buttons_start_y,
            ),
            vertical_margin,
        );

        for (id, text) in BUTTON_LABELS.into_iter().enumerate() {
            let slot = layout.next_slot(button_size.height);
            draw_button(
                target,
                text,
                slot.top_left,
                button_size,
                self.hot_idx == id as u32,
                self.active_idx == Some(id as u32),
            )?;
        }

        let slot = layout.next_slot(button_size.height);
        draw_para(target, &self.buffer, slot.top_left, button_size)?;

        Ok(())
    }
}

struct VerticalLayout {
    top_left: Point,
    margin: u32,
}

impl VerticalLayout {
    pub fn new(top_left: Point, margin: u32) -> Self {
        Self { top_left, margin }
    }

    pub fn next_slot(&mut self, height: u32) -> Rectangle {
        let slot = Rectangle::new(self.top_left, Size::new(0, height));
        self.top_left = self.top_left + Size::new(0, height + self.margin);

        slot
    }
}

struct Ui {
    focus_items_count: u32,
}


// impl Ui {
//     pub fn new() -> Self {
//         Self {
//             layout_pass: false,
//         } 
//     }

//     pub fn button(&mut self) -> bool {
//         false
//     }
// }
