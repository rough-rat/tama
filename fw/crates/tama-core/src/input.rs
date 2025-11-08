#[derive(Debug)]
pub struct Input {
    buttons: [ButtonState; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Up = 0,
    Down,
    Left,
    Right,
    A,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    JustPressed,
    Pressed,
    JustReleased,
    Released,
}

impl Input {
    pub fn new() -> Self {
        Self {
            buttons: [ButtonState::Released; 6],
        }
    }

    pub fn set_button(&mut self, button: Button, state: ButtonState) {
        self.buttons[button as usize] = state;
    }

    pub fn is_pressed(&self, button: Button) -> bool {
        let state = self.buttons[button as usize];
        state == ButtonState::JustPressed || state == ButtonState::Pressed
    }

    pub fn is_just_pressed(&self, button: Button) -> bool {
        let state = self.buttons[button as usize];
        state == ButtonState::JustPressed
    }
}
