use std::collections::HashMap;

use embedded_graphics::prelude::Size;
use embedded_graphics_simulator::sdl2::Keycode;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use tama_core::consts;
use tama_core::engine::Engine;
use tama_core::input::{Button, ButtonState};

mod log;

fn main() -> anyhow::Result<()> {
    let mut display =
        SimulatorDisplay::<consts::ColorType>::new(Size::new(consts::WIDTH, consts::HEIGHT));
    let settings = OutputSettingsBuilder::new().scale(2).pixel_spacing(0).build();

    let mut window = Window::new("tama-desktop", &settings);
    window.set_max_fps(30);
    let mut engine = Engine::new();
    window.update(&display);

    let mut button_pressed: HashMap<Button, bool> = HashMap::new();

    'running: loop {
        // there's a 100% a better way to handle input but idk, this is just for testing
        for (button, pressed) in button_pressed.iter() {
            engine.input_mut().set_button(
                *button,
                if *pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
            );
        }

        for event in window.events() {
            match event {
                SimulatorEvent::Quit => {
                    break 'running;
                }
                SimulatorEvent::KeyDown { keycode, repeat: false, .. } => {
                    let button = match keycode {
                        Keycode::W => Some(Button::Up),
                        Keycode::A => Some(Button::Left),
                        Keycode::S => Some(Button::Down),
                        Keycode::D => Some(Button::Right),
                        Keycode::J => Some(Button::A),
                        Keycode::K => Some(Button::B),
                        _ => None,
                    };

                    if let Some(button) = button {
                        engine
                            .input_mut()
                            .set_button(button, ButtonState::JustPressed);
                        button_pressed.insert(button, true);
                    }
                }
                SimulatorEvent::KeyUp { keycode, .. } => {
                    let button = match keycode {
                        Keycode::W => Some(Button::Up),
                        Keycode::A => Some(Button::Left),
                        Keycode::S => Some(Button::Down),
                        Keycode::D => Some(Button::Right),
                        Keycode::J => Some(Button::A),
                        Keycode::K => Some(Button::B),
                        _ => None,
                    };

                    if let Some(button) = button {
                        engine
                            .input_mut()
                            .set_button(button, ButtonState::JustReleased);
                        button_pressed.insert(button, false);
                    }
                }
                _ => (),
            }
        }

        engine.update();
        engine.render(&mut display)?;
        window.update(&display);
    }

    Ok(())
}
