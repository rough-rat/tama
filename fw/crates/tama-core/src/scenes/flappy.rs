use core::ops::Range;

use embedded_graphics::{
    Drawable as _,
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, Rectangle},
};
use heapless::Deque;
use rand::Rng;

use crate::{consts, engine::Context, scenes::Scene};

const SPACING: i32 = 100;
const PIPE_WIDTH: u32 = 32;
const GAP_HEIGHT_RANGE: Range<i32> = 64..128;
const GAP_CENTER_RANGE: Range<i32> = -64..64;

pub struct FlappyScene {
    pipes: Deque<Pipe, 8>,
}

impl FlappyScene {
    pub fn new() -> Self {
        Self {
            pipes: Default::default(),
        }
    }
}

impl Scene for FlappyScene {
    fn update(&mut self, ctx: &mut Context) {
        let scroll_speed = 1;

        if self.pipes.is_empty() || self.pipes.back().unwrap().x < consts::WIDTH as i32 - SPACING {
            self.pipes.push_back(Pipe {
                x: consts::WIDTH as i32,
                center_y: consts::HEIGHT as i32 / 2 + ctx.rng.random_range(GAP_CENTER_RANGE),
                gap_height: ctx.rng.random_range(GAP_HEIGHT_RANGE),
            }).expect("queue capacity isn't big enough for the pipe parameters");
        }

        for pipe in self.pipes.iter_mut() {
            pipe.x -= scroll_speed;
        }

        if let Some(front) = self.pipes.front()
            && front.x < -SPACING {
                self.pipes.pop_front();
            }
    }

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        // todo clear screen
        target.clear(consts::ColorType::WHITE)?;


        let black_fill = PrimitiveStyle::with_fill(consts::ColorType::BLACK);
        for pipe in self.pipes.iter() {
            // top pipe
            Rectangle::new(
                Point::new(pipe.x, 0),
                Size::new(
                    PIPE_WIDTH,
                    (pipe.center_y - pipe.gap_height / 2) as u32,
                ),
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
