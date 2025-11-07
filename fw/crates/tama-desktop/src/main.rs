use embedded_graphics::{prelude::Size};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window};
use tama_core::engine::Engine;
use tama_core::consts;


fn main() -> anyhow::Result<()> {

    let mut display = SimulatorDisplay::<consts::ColorType>::new(Size::new(consts::WIDTH, consts::HEIGHT));
    let settings = OutputSettingsBuilder::new().scale(2).build();

    let mut window = Window::new("tama-desktop", &settings);
    window.set_max_fps(30);
    let mut engine = Engine::new();
    window.update(&display);

    'running: loop {
        for event in window.events() {
            #[allow(clippy::single_match)]
            match event {
                SimulatorEvent::Quit => {
                    break 'running;
                },
                _ => (),
            }
        }
        engine.update();
        engine.render(&mut display)?;

        window.update(&display);
    }

    Ok(())
}
