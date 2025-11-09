use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point, Primitive, RgbColor},
    primitives::{Circle, PrimitiveStyle},
};

use crate::{assets, consts, engine::Context, gfx::Sprite, scenes::{Scene, UpdateResult}};

/// Very simple test scene
pub struct DvdScene {
    x: i32,
    y: i32,
    vel_x: i32,
    vel_y: i32,
    radius: u32,
}

impl DvdScene {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            x: consts::WIDTH as i32 / 2,
            y: consts::HEIGHT as i32 / 2,
            vel_x: 1,
            vel_y: -1,
            radius: 64,
        }
    }
}

impl Scene for DvdScene {
    fn update(&mut self, _ctx: &mut Context) -> UpdateResult {
        self.x += self.vel_x;
        if self.x <= self.radius as i32 || self.x >= (consts::WIDTH - self.radius) as i32 {
            self.vel_x = -self.vel_x;
        }

        self.y += self.vel_y;
        if self.y <= self.radius as i32 || self.y >= (consts::HEIGHT - self.radius) as i32 {
            self.vel_y = -self.vel_y;
        }

        UpdateResult::None
    }

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        target.clear(consts::ColorType::WHITE)?;

        // let fill = PrimitiveStyle::with_fill(consts::ColorType::RED);
        // Circle::with_center(Point::new(self.x, self.y), self.radius * 2)
        //     .into_styled(fill)
        //     .draw(target)?;

        Sprite::new(&*assets::images::PAPAJ, Point::new(self.x - self.radius as i32, self.y - self.radius as i32)).draw(target)?;

        Ok(())
    }
}
