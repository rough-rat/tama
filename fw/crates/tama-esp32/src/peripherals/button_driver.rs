//! Button driver with thread-safe state management.
//! 
//! This driver handles button input from GPIOs and provides a synchronized
//! interface for reading button states. The design separates GPIO reading
//! from state consumption, allowing for future interrupt-based or 
//! thread-based input handling.

use esp_idf_hal::gpio::{AnyInputPin, Input, PinDriver};
use std::sync::{Arc, Mutex};
use tama_core::input::{Button, ButtonState, Input as EngineInput};

use super::ButtonPeripherals;

/// Number of buttons in the system
const NUM_BUTTONS: usize = 7;

/// Raw button state read from GPIO (active low)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawButtonState {
    /// True if button is currently pressed (GPIO is low)
    pressed: bool,
}

/// Internal button state with edge detection
#[derive(Debug, Clone, Copy)]
struct InternalButtonState {
    current: bool,
    previous: bool,
}

impl InternalButtonState {
    fn new() -> Self {
        Self {
            current: false,
            previous: false,
        }
    }

    /// Update state and return the resulting ButtonState
    fn update(&mut self, pressed: bool) -> ButtonState {
        self.previous = self.current;
        self.current = pressed;

        match (self.previous, self.current) {
            (false, true) => ButtonState::JustPressed,
            (true, true) => ButtonState::Pressed,
            (true, false) => ButtonState::JustReleased,
            (false, false) => ButtonState::Released,
        }
    }
}

/// Thread-safe shared button states
/// 
/// This structure holds the synchronized button states that can be
/// written to by GPIO reading code and read by the main game loop.
struct SharedButtonStates {
    /// Raw GPIO states (written by update(), read by apply_to_input())
    raw_states: [RawButtonState; NUM_BUTTONS],
}

impl SharedButtonStates {
    fn new() -> Self {
        Self {
            raw_states: [RawButtonState { pressed: false }; NUM_BUTTONS],
        }
    }
}

/// Button driver that manages all game buttons.
/// 
/// The driver maintains GPIO pin drivers and provides thread-safe
/// button state management. Currently uses polling via `update()`,
/// but the architecture supports future migration to interrupts.
/// 
/// # Button Mapping
/// - A: GPIO15
/// - B: GPIO7  
/// - Up: GPIO8
/// - Down: GPIO18
/// - Left: GPIO17
/// - Right: GPIO16
/// - Pwr (BOOT): GPIO0
pub struct ButtonDriver<'a> {
    // GPIO pin drivers
    pin_a: PinDriver<'a, AnyInputPin, Input>,
    pin_b: PinDriver<'a, AnyInputPin, Input>,
    pin_up: PinDriver<'a, AnyInputPin, Input>,
    pin_down: PinDriver<'a, AnyInputPin, Input>,
    pin_left: PinDriver<'a, AnyInputPin, Input>,
    pin_right: PinDriver<'a, AnyInputPin, Input>,
    pin_boot: PinDriver<'a, AnyInputPin, Input>,
    
    /// Shared button states (synchronized for thread safety)
    shared_states: Arc<Mutex<SharedButtonStates>>,
    
    /// Internal edge detection state (local to whoever calls apply_to_input)
    edge_states: [InternalButtonState; NUM_BUTTONS],
}

impl<'a> ButtonDriver<'a> {
    /// Creates a new button driver from button peripherals.
    /// 
    /// All buttons are configured as inputs with internal pull-up resistors.
    /// Buttons are active-low (pressed = GPIO low).
    pub fn new(peripherals: ButtonPeripherals) -> Self {
        // Configure all pins as inputs with pull-up
        let pin_a = PinDriver::input(peripherals.btn_a).unwrap();
        let pin_b = PinDriver::input(peripherals.btn_b).unwrap();
        let pin_up = PinDriver::input(peripherals.btn_up).unwrap();
        let pin_down = PinDriver::input(peripherals.btn_down).unwrap();
        let pin_left = PinDriver::input(peripherals.btn_left).unwrap();
        let pin_right = PinDriver::input(peripherals.btn_right).unwrap();
        let pin_boot = PinDriver::input(peripherals.btn_boot).unwrap();

        log::info!("ButtonDriver initialized with 7 buttons");

        Self {
            pin_a,
            pin_b,
            pin_up,
            pin_down,
            pin_left,
            pin_right,
            pin_boot,
            shared_states: Arc::new(Mutex::new(SharedButtonStates::new())),
            edge_states: [InternalButtonState::new(); NUM_BUTTONS],
        }
    }

    /// Reads current GPIO states and updates the shared button states.
    /// 
    /// This method should be called periodically (e.g., once per frame)
    /// to poll the button states. In the future, this could be replaced
    /// with interrupt-driven updates.
    /// 
    /// Buttons are active-low: GPIO low = pressed.
    pub fn update(&self) {
        let mut states = self.shared_states.lock().unwrap();
        
        // Read all GPIO pins (active low - is_low() means pressed)
        states.raw_states[Button::A as usize].pressed = self.pin_a.is_low();
        states.raw_states[Button::B as usize].pressed = self.pin_b.is_low();
        states.raw_states[Button::Up as usize].pressed = self.pin_up.is_low();
        states.raw_states[Button::Down as usize].pressed = self.pin_down.is_low();
        states.raw_states[Button::Left as usize].pressed = self.pin_left.is_low();
        states.raw_states[Button::Right as usize].pressed = self.pin_right.is_low();
        states.raw_states[Button::Pwr as usize].pressed = self.pin_boot.is_low();
    }

    /// Applies the current button states to the engine input.
    /// 
    /// This method reads the shared button states, performs edge detection,
    /// and updates the engine's input state accordingly.
    /// 
    /// # Arguments
    /// * `input` - Mutable reference to the engine's Input struct
    pub fn apply_to_input(&mut self, input: &mut EngineInput) {
        // Read shared states
        let states = self.shared_states.lock().unwrap();
        let raw_states = states.raw_states;
        drop(states); // Release lock before processing

        // Process each button with edge detection
        let buttons = [
            Button::A,
            Button::B,
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
            Button::Pwr,
        ];

        for button in buttons {
            let idx = button as usize;
            let pressed = raw_states[idx].pressed;
            let state = self.edge_states[idx].update(pressed);
            input.set_button(button, state);
        }
    }

    /// Returns a clone of the shared states Arc for use in other contexts.
    /// 
    /// This allows other threads or interrupt handlers to update button
    /// states directly in the future.
    #[allow(dead_code)]
    pub fn get_shared_states(&self) -> Arc<Mutex<SharedButtonStates>> {
        Arc::clone(&self.shared_states)
    }
}
