use core::ops::Range;

use embedded_graphics::{
    Drawable as _, image::Image, prelude::{DrawTarget, Point, Primitive, RgbColor, Size}, primitives::{Circle, PrimitiveStyle, Rectangle}
};
use heapless::Deque;
use rand::Rng;

use crate::{
    assets, consts, engine::Context, gfx::Sprite, input::Button, scenes::{Scene, SceneWrapper, UpdateResult, menu::MenuScene}
};

const SCROLL_SPEED: i32 = 1;
const SPACING: i32 = 100;
const PIPE_WIDTH: u32 = 32;
const GAP_HEIGHT_RANGE: Range<i32> = 64..128;
const GAP_CENTER_RANGE: Range<i32> = -64..64;

const PLAYER_RADIUS: u32 = 8;
const PLAYER_GRAVITY: f32 = 0.7;
const PLAYER_JUMP_VELOCITY: f32 = 7.0;

pub struct FlappyScene {
    pipes: Deque<Pipe, 8>,

    player_x: i32,
    player_y: f32,
    player_y_speed: f32,
}

impl FlappyScene {
    pub fn new() -> Self {
        Self {
            pipes: Default::default(),
            player_x: 32,
            player_y: (consts::HEIGHT / 2) as f32,
            player_y_speed: 0.0,
        }
    }
}

impl Scene for FlappyScene {
    fn update(&mut self, ctx: &mut Context) -> UpdateResult {
        // Pipes
        if self.pipes.is_empty() || self.pipes.back().unwrap().x < consts::WIDTH as i32 - SPACING {
            self.pipes
                .push_back(Pipe {
                    x: consts::WIDTH as i32,
                    center_y: consts::HEIGHT as i32 / 2 + ctx.rng.random_range(GAP_CENTER_RANGE),
                    gap_height: ctx.rng.random_range(GAP_HEIGHT_RANGE),
                })
                .expect("queue capacity isn't big enough for the pipe parameters");
        }

        for pipe in self.pipes.iter_mut() {
            pipe.x -= SCROLL_SPEED;
        }

        if let Some(front) = self.pipes.front()
            && front.x < -SPACING
        {
            self.pipes.pop_front();
        }

        // player
        if ctx.input.is_just_pressed(Button::Up) {
            self.player_y_speed = -PLAYER_JUMP_VELOCITY;
            ctx.output.play_tone(40, 20);
        }

        self.player_y += self.player_y_speed;
        self.player_y_speed += PLAYER_GRAVITY;

        // collision
        let radius = PLAYER_RADIUS as i32;

        let is_in_bounds = self.player_y as i32 - radius >= 0
            && self.player_y as i32 + radius <= consts::HEIGHT as i32;

        if !is_in_bounds {
            ctx.output.play_tone(60, 500);
            return UpdateResult::ChangeScene(SceneWrapper::from(MenuScene::new()));
        }

        for pipe in self.pipes.iter() {
            // stop if the pipe is too far
            if pipe.x > self.player_x + radius {
                break;
            }

            let has_x_overlap = self.player_x + radius > pipe.x
                && self.player_x - radius < pipe.x + PIPE_WIDTH as i32;
            let y = self.player_y as i32;
            let has_y_overlap = y - radius < pipe.center_y - pipe.gap_height / 2
                || y + radius > pipe.center_y + pipe.gap_height / 2;

            if has_x_overlap && has_y_overlap {
                ctx.output.play_tone(58, 500);
                return UpdateResult::ChangeScene(SceneWrapper::from(MenuScene::new()));
            }
        }

        UpdateResult::None
    }

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        target.clear(consts::ColorType::WHITE)?;

        let black_fill = PrimitiveStyle::with_fill(consts::ColorType::BLACK);
        let green_fill = PrimitiveStyle::with_fill(consts::ColorType::GREEN);

        for pipe in self.pipes.iter() {
            // top pipe
            Rectangle::new(
                Point::new(pipe.x, 0),
                Size::new(PIPE_WIDTH, (pipe.center_y - pipe.gap_height / 2) as u32),
            )
            .into_styled(black_fill)
            .draw(target)?;

            // bottom pipe
            Rectangle::new(
                Point::new(pipe.x, pipe.center_y + pipe.gap_height / 2),
                Size::new(
                    PIPE_WIDTH,
                    (consts::HEIGHT as i32 - pipe.center_y - pipe.gap_height / 2) as u32,
                ),
            )
            .into_styled(black_fill)
            .draw(target)?;

            // player
            // Circle::with_center(
            //     Point::new(self.player_x, self.player_y as i32),
            //     PLAYER_RADIUS * 2,
            // )
            // .into_styled(green_fill)
            // .draw(target)?;

            Sprite::new(&*assets::images::PAPAJ_SMOL, Point::new(self.player_x - PLAYER_RADIUS as i32, self.player_y as i32 - PLAYER_RADIUS as i32)).draw(target)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Pipe {
    x: i32,
    center_y: i32,
    gap_height: i32,
}
