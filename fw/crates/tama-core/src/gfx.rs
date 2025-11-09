use embedded_graphics::{
    Drawable, Pixel, pixelcolor::{Rgb555, Rgb565, Rgb888}, prelude::{DrawTarget, PixelColor, Point}
};
use tinybmp::Bmp;

pub struct Sprite<'a, 'b, C>
where
    C: PixelColor + From<Rgb555> + From<Rgb565> + From<Rgb888>,
{
    bmp_image: &'a Bmp<'b, C>,
    position: Point,
    transparency_key: C,
}

impl<'bmp_image, 'bmp_data, C> Sprite<'bmp_image, 'bmp_data, C>
where
    C: PixelColor + From<Rgb555> + From<Rgb565> + From<Rgb888>,
{
    pub fn new(bmp: &'bmp_image Bmp<'bmp_data, C>, position: Point) -> Self {
        Self {
            bmp_image: bmp,
            position,
            transparency_key: C::from(Rgb888::new(0xff, 0, 0xff)), // CYAN
        }
    }
}

impl<'bmp_image, 'bmp_data, C> Drawable for Sprite<'bmp_image, 'bmp_data, C>
where
    C: PixelColor + From<Rgb555> + From<Rgb565> + From<Rgb888>,
{
    type Color = C;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        // This is probably horribly inefficient
        for pixel in self.bmp_image.pixels() {
            if pixel.1 == self.transparency_key {
                continue;
            }

            let x = pixel.0.x + self.position.x;
            let y = pixel.0.y + self.position.y;
            Pixel(Point::new(x, y), pixel.1).draw(target)?; 
        }

        Ok(())
    }
}
