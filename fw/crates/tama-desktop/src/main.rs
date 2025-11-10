use std::collections::HashMap;

use embedded_graphics::prelude::Size;
use embedded_graphics_simulator::sdl2::Keycode;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use tama_core::consts;
use tama_core::engine::Engine;
use tama_core::input::{Button, ButtonState};

use tama_core::input::SensorType;

mod buzzer;
mod mock_hw_tui;

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

fn generate_mock_hw_data(engine: &mut Engine, tui: &mock_hw_tui::MockHwTui) {
    // Get sensor values from TUI
    let sensors = tui.get_sensor_state();
    let time_ms = 0; // TODO: get actual time
    
    engine.input_mut().update_sensor(SensorType::BatteryVoltage, sensors.battery_voltage, time_ms);
    engine.input_mut().update_sensor(SensorType::Thermometer, sensors.temperature, time_ms);
    engine.input_mut().update_sensor(SensorType::LightSensor, sensors.light_level, time_ms);
    engine.input_mut().update_sensor(SensorType::Accelerometer, sensors.accelerometer, time_ms);
    engine.input_mut().update_sensor(SensorType::MicLoudness, sensors.mic_loudness, time_ms);
}

fn main() -> anyhow::Result<()> {
    // Initialize the Mock Hardware TUI (also sets up the logger)
    let tui = mock_hw_tui::MockHwTui::new()?;
    log::info!("Tama Desktop Simulator started");
    
    // Create the desktop buzzer (handles audio asynchronously)
    let buzzer = Box::new(buzzer::DesktopBuzzer::new());
    log::info!("Audio buzzer initialized");

    let mut display =
        SimulatorDisplay::<consts::ColorType>::new(Size::new(consts::WIDTH, consts::HEIGHT));
    let settings = OutputSettingsBuilder::new().scale(2).pixel_spacing(0).build();

    let mut window = Window::new("tama-desktop", &settings);
    window.set_max_fps(30);
    let mut engine = Engine::with_buzzer(buzzer);
    let mut button_pressed: HashMap<Button, bool> = HashMap::new();
    
    log::info!("Engine and display initialized");

    'running: loop {
        window.update(&display);

        if !handle_simulator_events(&mut engine, &mut window, &mut button_pressed) {
            log::info!("Simulator window closed");
            break 'running;
        } //TODO verbose exit handling        

        generate_mock_hw_data(&mut engine, &tui);
        engine.update();
        engine.render(&mut display)?;
    }

    Ok(())
}
