const MOVING_AVG_ALPHA: f32 = 0.1;

#[derive(PartialEq)]
#[derive(Debug)]
pub enum SensorState {
    Uninitialized = 0,
    Normal,
    Event,
    SensorError,
}

#[derive(PartialEq)]
#[derive(Debug)]
pub struct SensorData {
    raw: f32,
    moving_avg: f32,
    state: SensorState,
    last_updated_ms: u32,
}

impl SensorData {
    pub fn new() -> Self {
        Self {
            raw: 0.0,
            moving_avg: 0.0,
            state: SensorState::Uninitialized,
            last_updated_ms: 0,
        }
    }

    pub fn update(&mut self, raw_value: f32, current_time_ms: u32) {
        match self.state {
            SensorState::SensorError | SensorState::Uninitialized => {
                // Debug: sensor not initialized or in error state
                return;
            }
            SensorState::Event | SensorState::Normal => {
                self.raw = raw_value;
                if self.state == SensorState::Uninitialized {
                    self.moving_avg = raw_value;
                    self.state = SensorState::Normal;
                } else {
                    self.moving_avg = MOVING_AVG_ALPHA * raw_value + (1.0 - MOVING_AVG_ALPHA) * self.moving_avg;
                }
                self.last_updated_ms = current_time_ms;
            }
        }
    }
}

pub enum SensorType {
    BatteryVoltage = 0,
    Thermometer,
    LightSensor,
    Accelerometer,
    MicLoudness,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Up = 0,
    Down,
    Left,
    Right,
    A,
    B,
    Pwr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    JustPressed,
    Pressed,
    JustReleased,
    Released,
}

#[derive(Debug)]
pub struct Input {
    buttons: [ButtonState; 7],
    sensors: [SensorData; 5],
}

impl Input {
    pub fn new() -> Self {
        Self {
            buttons: [ButtonState::Released; 7],
            sensors: [
                SensorData::new(),
                SensorData::new(),
                SensorData::new(),
                SensorData::new(),
                SensorData::new(),
            ], //TODO
        }
    }

    pub fn update_sensor(
        &mut self,
        sensor_type: SensorType,
        raw_value: f32,
        current_time_ms: u32,
    ) {
        let sensor = &mut self.sensors[sensor_type as usize];
        sensor.update(raw_value, current_time_ms);
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
