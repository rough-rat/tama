use core::fmt::Write as _;
use embedded_graphics::{
    Drawable as _,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size, WebColors},
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text, renderer::TextRenderer},
};
use heapless::String;

use crate::{consts, engine::DrawContext, input::SensorType};

// Drawing

pub fn draw_top_bar<DT>(target: &mut DT, ctx: &DrawContext) -> Result<(), DT::Error>
where
    DT: DrawTarget<Color = consts::ColorType>,
{
    let width = target.bounding_box().size.width;
    Rectangle::new(Point::zero(), Size::new(width, 16))
        .into_styled(PrimitiveStyle::with_fill(consts::ColorType::BLACK))
        .draw(target)?;

    let text_y = 10; // not sure why this is calculated like that
    let text_style = MonoTextStyle::new(&FONT_6X10, consts::ColorType::WHITE);
    Text::with_alignment(
        "21:37",
        Point::new(width as i32 / 2, text_y),
        text_style,
        Alignment::Center,
    )
    .draw(target)?;

    let battery_voltage = ctx.input.get_sensor_value(SensorType::BatteryLevel);
    let mut battery_text = String::<16>::new();
    let _ = write!(battery_text, "{:.2}V", battery_voltage);
    Text::with_alignment(
        battery_text.as_str(),
        Point::new((width as i32) - 20, text_y),
        text_style,
        Alignment::Right,
    )
    .draw(target)?;

    Ok(())
}

pub fn draw_button<DT>(
    target: &mut DT,
    text: &str,
    top_left: Point,
    size: Size,
    focused: bool,
    active: bool,
) -> Result<(), DT::Error>
where
    DT: DrawTarget<Color = consts::ColorType>,
{
    let fill_color = if active {
        consts::ColorType::CSS_DARK_GRAY
    } else {
        consts::ColorType::CSS_LIGHT_GRAY
    };

    Rectangle::new(top_left, size)
        .into_styled(PrimitiveStyle::with_fill(fill_color))
        .draw(target)?;

    if focused {
        Rectangle::new(top_left, size)
            .into_styled(PrimitiveStyle::with_stroke(consts::ColorType::CSS_GRAY, 1))
            .draw(target)?;
    }

    let text_style = MonoTextStyle::new(&FONT_6X10, consts::ColorType::BLACK);
    let pos = Point::new(
        top_left.x + size.width as i32 / 2,
        top_left.y + size.height as i32 / 2 + 2,
    );
    Text::with_alignment(text, pos, text_style, Alignment::Center).draw(target)?;

    Ok(())
}


pub fn draw_para<DT>(
    target: &mut DT,
    text: &str,
    top_left: Point,
    width: Size,
) -> Result<(), DT::Error>
where
    DT: DrawTarget<Color = consts::ColorType>,
{
    let text_style = MonoTextStyle::new(&FONT_6X10, consts::ColorType::BLACK);
    Text::with_alignment(text, top_left, text_style, Alignment::Left).draw(target)?;
    
    // let mut start = 0;
    // let mut end = 0;

    // Text::with_alignment(text, top_left, text_style, Alignment::Left)
    
    Ok(())
}
