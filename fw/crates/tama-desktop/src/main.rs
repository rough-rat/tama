use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use embedded_graphics::prelude::{DrawTarget, Point, Size};
use embedded_graphics::Pixel;
use embedded_graphics_simulator::sdl2::Keycode;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};

use embedded_graphics::prelude::RgbColor;
use tama_core::consts;
use tama_core::engine::{Engine, TimeInfo};
use tama_core::input::{Button, ButtonState};
use tama_core::input::SensorType;
use tama_core::notice;

mod buzzer;
mod log_capture;
mod mock_hw_tui;

const TOP_CORNER_RADIUS_PX: i32 = 40;
const BOTTOM_CORNER_RADIUS_PX: i32 = 25;

fn handle_simulator_events(
    engine: &mut Engine, 
    window: &mut Window, 
    button_pressed: &mut HashMap<Button, bool>
) -> bool {
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
                    return false;
                }
                SimulatorEvent::KeyDown { keycode, repeat: false, .. } => {
                    let button = match keycode {
                        Keycode::W => Some(Button::Up),
                        Keycode::A => Some(Button::Left),
                        Keycode::S => Some(Button::Down),
                        Keycode::D => Some(Button::Right),
                        Keycode::J => Some(Button::A),
                        Keycode::K => Some(Button::B),
                        Keycode::Escape => {
                            log::info!("Escape pressed, exiting simulator.");
                            return false;
                        }
                        _ => None,
                    };

                    if let Some(button) = button {
                        log::debug!("Button pressed: {:?}", button);
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
    true

}

fn apply_rounded_corner_mask(display: &mut SimulatorDisplay<consts::ColorType>) {
    let width = consts::WIDTH as i32;
    let height = consts::HEIGHT as i32;
    let top_r = TOP_CORNER_RADIUS_PX;
    let bottom_r = BOTTOM_CORNER_RADIUS_PX;
    let top_r_sq = top_r * top_r;
    let bottom_r_sq = bottom_r * bottom_r;

    let mut pixels: Vec<Pixel<consts::ColorType>> = Vec::new();

    for y in 0..top_r {
        for x in 0..top_r {
            let dx = x - top_r;
            let dy = y - top_r;
            if dx * dx + dy * dy > top_r_sq {
                pixels.push(Pixel(Point::new(x, y), consts::ColorType::BLACK));
            }
        }
    }

    for y in 0..top_r {
        for x in (width - top_r)..width {
            let dx = x - (width - 1 - top_r);
            let dy = y - top_r;
            if dx * dx + dy * dy > top_r_sq {
                pixels.push(Pixel(Point::new(x, y), consts::ColorType::BLACK));
            }
        }
    }

    for y in (height - bottom_r)..height {
        for x in 0..bottom_r {
            let dx = x - bottom_r;
            let dy = y - (height - 1 - bottom_r);
            if dx * dx + dy * dy > bottom_r_sq {
                pixels.push(Pixel(Point::new(x, y), consts::ColorType::BLACK));
            }
        }
    }

    for y in (height - bottom_r)..height {
        for x in (width - bottom_r)..width {
            let dx = x - (width - 1 - bottom_r);
            let dy = y - (height - 1 - bottom_r);
            if dx * dx + dy * dy > bottom_r_sq {
                pixels.push(Pixel(Point::new(x, y), consts::ColorType::BLACK));
            }
        }
    }

    let _ = display.draw_iter(pixels);
}

fn generate_mock_hw_data(engine: &mut Engine, tui: &mock_hw_tui::MockHwTui) {
    // Get sensor values from TUI
    let sensors = tui.get_sensor_state();
    let time_ms = 0; // TODO: get actual time
    
    engine.input_mut().update_sensor(SensorType::BatteryLevel, sensors.battery_level, time_ms);
    engine.input_mut().update_sensor(SensorType::Thermometer, sensors.temperature, time_ms);
    engine.input_mut().update_sensor(SensorType::LightSensor, sensors.light_level, time_ms);
    engine.input_mut().update_sensor(SensorType::Accelerometer, sensors.accelerometer, time_ms);
    engine.input_mut().update_sensor(SensorType::MicLoudness, sensors.mic_loudness, time_ms);
}

fn system_time_info() -> TimeInfo {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seconds_today = (now.as_secs() % 86_400) as u32;

    TimeInfo {
        minutes: seconds_today / 60,
        seconds: seconds_today % 60,
    }
}

fn main() -> anyhow::Result<()> {
    // Initialize log capture system first
    log_capture::init(log::LevelFilter::Info);
    
    notice!("Tama Desktop starting...");

    // Initialize the Mock Hardware TUI
    let tui = mock_hw_tui::MockHwTui::new()?;
    notice!("Mock hardware TUI initialized");
    
    // Create the desktop buzzer (handles audio asynchronously)
    let buzzer = Box::new(buzzer::DesktopBuzzer::new());
    notice!("Audio buzzer initialized");

    let mut display =
        SimulatorDisplay::<consts::ColorType>::new(Size::new(consts::WIDTH, consts::HEIGHT));
    let settings = OutputSettingsBuilder::new().scale(2).pixel_spacing(0).build();

    let mut window = Window::new("tama-desktop", &settings);
    window.set_max_fps(30);
    let mut engine = Engine::with_buzzer(buzzer);
    let mut button_pressed: HashMap<Button, bool> = HashMap::new();
    
    notice!("Engine and display initialized");

    'running: loop {
        window.update(&display);

        if !handle_simulator_events(&mut engine, &mut window, &mut button_pressed) {
            log::info!("Simulator window closed");
            break 'running;
        }

        generate_mock_hw_data(&mut engine, &tui);
        
        // Push recent log entries to engine for on-screen display
        engine.push_log_entries(log_capture::recent_log_entries(16));
        engine.set_time(system_time_info());
        
        engine.update();
        engine.render(&mut display)?;
        apply_rounded_corner_mask(&mut display);
    }

    Ok(())
}
