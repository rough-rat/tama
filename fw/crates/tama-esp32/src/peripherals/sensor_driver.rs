//! Sensor Driver - Light, microphone, and I2C sensor management
//!
//! This module handles:
//! - Light sensor via ADC (GPIO2, with enable on GPIO40)
//! - Microphone via ADC (GPIO1)
//! - I2C sensors (accelerometer, temperature/humidity)
//!
//! Battery monitoring is handled by PowerControl.

use std::sync::{Arc, Mutex};

use esp_idf_hal::adc::oneshot::AdcChannelDriver;
use esp_idf_hal::gpio::{self, AnyInputPin, AnyOutputPin, Output, PinDriver};

use tama_core::input::{Input, SensorType};

use crate::peripherals::adc_bus::{AdcBus, SharedAdc1Driver};
use crate::peripherals::power_control::PowerControl;
use crate::peripherals::sensors_i2c::{I2cBusConfig, I2cSensorBus};
use crate::peripherals::SensorPeripherals;

/// Shared sensor state for thread-safe access
#[derive(Default, Clone)]
pub struct SharedSensorState {
    pub light_sensor: f32,      // Light level (0.0 - 1.0 normalized)
    pub mic_loudness: f32,      // Microphone level (0.0 - 1.0 normalized)
    pub thermometer: f32,       // Temperature in Celsius (from I2C)
    pub accelerometer: f32,     // Placeholder value (from I2C)
}

type SharedState = Arc<Mutex<SharedSensorState>>;

/// Sensor driver that handles light, microphone, and I2C sensors
/// 
/// Sensors:
/// - LightSensor: ADC on GPIO2, enable via GPIO40
/// - MicLoudness: ADC on GPIO1
/// - Thermometer: I2C HDC1080 (stub for now)
/// - Accelerometer: I2C MMA8451 (stub for now)
///
/// Note: Battery monitoring is handled by PowerControl
pub struct SensorDriver<'d> {
    // Light sensor: GPIO2, with enable on GPIO40
    light_channel: AdcChannelDriver<'d, gpio::Gpio2, SharedAdc1Driver<'d>>,
    light_enable: PinDriver<'d, AnyOutputPin, Output>,
    
    // Microphone: GPIO1
    mic_channel: AdcChannelDriver<'d, gpio::Gpio1, SharedAdc1Driver<'d>>,
    
    // I2C sensor bus for accelerometer and temp/humidity
    i2c_bus: I2cSensorBus<'d>,
    
    // Accelerometer interrupt pin (unused for now)
    #[allow(dead_code)]
    acc_int1: AnyInputPin,
    
    // Shared state for thread-safe access
    state: SharedState,
}

impl<'d> SensorDriver<'d> {
    /// Create a new SensorDriver with the given peripherals
    /// 
    /// Requires an AdcBus for creating ADC channels.
    pub fn new(adc_bus: &AdcBus<'d>, peripherals: SensorPeripherals) -> Self {
        // Light sensor channel (GPIO2)
        let light_channel = adc_bus.create_light_channel(peripherals.light_sensor_pin);
        
        // Light sensor enable pin (GPIO40) - start disabled
        let mut light_enable = PinDriver::output(peripherals.light_sensor_enable)
            .expect("Failed to create light enable pin");
        light_enable.set_low().ok();
        
        // Microphone channel (GPIO1)
        let mic_channel = adc_bus.create_mic_channel(peripherals.mic_pin);
        
        // Initialize I2C sensor bus
        let i2c_config = I2cBusConfig::default();
        let i2c_bus = I2cSensorBus::new(
            peripherals.i2c,
            peripherals.i2c_sda,
            peripherals.i2c_scl,
            &i2c_config,
        ).expect("Failed to create I2C sensor bus");
        
        log::info!("Sensor driver initialized");
        
        Self {
            light_channel,
            light_enable,
            mic_channel,
            i2c_bus,
            acc_int1: peripherals.acc_int1,
            state: Arc::new(Mutex::new(SharedSensorState::default())),
        }
    }
    
    /// Scan the I2C rail for connected devices
    /// 
    /// Returns a vector of I2C addresses that responded.
    /// Use this at startup to verify sensor presence.
    pub fn scan_i2c_rail(&mut self) -> Vec<u8> {
        self.i2c_bus.scan()
    }
    
    /// Scan the I2C rail and return a human-readable report
    /// 
    /// Returns a formatted string describing all found devices.
    pub fn scan_i2c_rail_report(&mut self) -> String {
        self.i2c_bus.scan_report()
    }
    
    /// Get a clone of the shared state handle
    pub fn shared_state(&self) -> SharedState {
        self.state.clone()
    }
    
    /// Update all sensor readings (except battery - use PowerControl)
    /// 
    /// Call this periodically from the main loop
    pub fn update(&mut self) {
        let mut state = self.state.lock().unwrap();
        
        // Read light sensor (enable first, then read)
        self.light_enable.set_high().ok();
        // Small delay would be ideal here, but for simplicity we read immediately
        if let Ok(raw) = self.light_channel.read_raw() {
            // Normalize to 0.0 - 1.0 range
            state.light_sensor = raw as f32 / 4095.0;
        }
        self.light_enable.set_low().ok();

        log::trace!("Light sensor: {:.3}", state.light_sensor);
        
        // Read microphone level
        if let Ok(raw) = self.mic_channel.read_raw() {
            // Normalize to 0.0 - 1.0 range
            state.mic_loudness = raw as f32 / 4095.0;
        }
        
        // Thermometer - I2C stub, return room temperature
        state.thermometer = 20.0;
        
        // Accelerometer - I2C stub, return 0
        state.accelerometer = 0.0;
    }
    
    /// Apply sensor readings to the engine's input system
    /// 
    /// Also reads battery level from PowerControl to include in input.
    pub fn apply_to_input(&self, input: &mut Input, power: &PowerControl, current_time_ms: u32) {
        let state = self.state.lock().unwrap();
        
        // Get battery from power controller
        let battery_voltage = power.get_battery_voltage();
        
        // Update all sensors in the engine's input system
        input.update_sensor(SensorType::BatteryLevel, battery_voltage, current_time_ms);
        input.update_sensor(SensorType::Thermometer, state.thermometer, current_time_ms);
        input.update_sensor(SensorType::LightSensor, state.light_sensor, current_time_ms);
        input.update_sensor(SensorType::Accelerometer, state.accelerometer, current_time_ms);
        input.update_sensor(SensorType::MicLoudness, state.mic_loudness, current_time_ms);
    }
    
    /// Get current light sensor reading (0.0 - 1.0)
    pub fn get_light_level(&self) -> f32 {
        self.state.lock().unwrap().light_sensor
    }
    
    /// Get current microphone level (0.0 - 1.0)
    pub fn get_mic_level(&self) -> f32 {
        self.state.lock().unwrap().mic_loudness
    }
    
    /// Get current temperature reading (from I2C sensor)
    pub fn get_temperature(&self) -> f32 {
        self.state.lock().unwrap().thermometer
    }
}
