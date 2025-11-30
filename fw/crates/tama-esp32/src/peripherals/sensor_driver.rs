use std::sync::{Arc, Mutex};

#[allow(deprecated)]
use esp_idf_hal::adc::attenuation::DB_11;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::gpio::{self, AnyInputPin, AnyOutputPin, PinDriver, Output};

use tama_core::input::{Input, SensorType};

use crate::peripherals::SensorPeripherals;
use crate::peripherals::sensors_i2c::{I2cSensorBus, I2cBusConfig};

/// Shared sensor state for thread-safe access
#[derive(Default, Clone)]
pub struct SharedSensorState {
    pub battery_voltage: f32,   // 0.0 - 4.2V (or calculated percentage)
    pub thermometer: f32,       // Temperature in Celsius
    pub light_sensor: f32,      // Light level (0.0 - 1.0 normalized)
    pub accelerometer: f32,     // Placeholder value
    pub mic_loudness: f32,      // Microphone level (0.0 - 1.0 normalized)
}

type SharedState = Arc<Mutex<SharedSensorState>>;

// Type alias for the ADC driver wrapped in Arc
// All our sensor pins are on ADC1
type SharedAdcDriver<'d> = Arc<AdcDriver<'d, esp_idf_hal::adc::ADC1>>;

/// Sensor driver that handles ADC readings for various sensors
/// 
/// Sensors:
/// - BatteryLevel: ADC on GPIO4, 0.5 voltage divider
/// - Thermometer: I2C (stub for now)
/// - LightSensor: ADC on GPIO2, enable via GPIO40
/// - Accelerometer: I2C (stub for now)
/// - MicLoudness: ADC on GPIO1
pub struct SensorDriver<'d> {
    // Battery voltage: GPIO4 with 0.5 voltage divider
    battery_channel: AdcChannelDriver<'d, gpio::Gpio4, SharedAdcDriver<'d>>,
    
    // Light sensor: GPIO2, with enable on GPIO40
    light_channel: AdcChannelDriver<'d, gpio::Gpio2, SharedAdcDriver<'d>>,
    light_enable: PinDriver<'d, AnyOutputPin, Output>,
    
    // Microphone: GPIO1
    mic_channel: AdcChannelDriver<'d, gpio::Gpio1, SharedAdcDriver<'d>>,
    
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
    pub fn new(peripherals: SensorPeripherals) -> Self {
        // Initialize ADC1 driver (wrapped in Arc for sharing)
        let adc = Arc::new(
            AdcDriver::new(peripherals.adc1).expect("Failed to create ADC driver")
        );
        
        // ADC channel config with 11dB attenuation for ~0-3.3V range
        #[allow(deprecated)]
        let config = AdcChannelConfig {
            attenuation: DB_11,
            ..Default::default()
        };
        
        // Battery channel (GPIO4)
        let battery_channel = AdcChannelDriver::new(adc.clone(), peripherals.battery_pin, &config)
            .expect("Failed to create battery ADC channel");
        
        // Light sensor channel (GPIO2)
        let light_channel = AdcChannelDriver::new(adc.clone(), peripherals.light_sensor_pin, &config)
            .expect("Failed to create light sensor ADC channel");
        
        // Light sensor enable pin (GPIO40) - start disabled
        let mut light_enable = PinDriver::output(peripherals.light_sensor_enable)
            .expect("Failed to create light enable pin");
        light_enable.set_low().ok();
        
        // Microphone channel (GPIO1)
        let mic_channel = AdcChannelDriver::new(adc, peripherals.mic_pin, &config)
            .expect("Failed to create microphone ADC channel");
        
        // Initialize I2C sensor bus
        let i2c_config = I2cBusConfig::default();
        let i2c_bus = I2cSensorBus::new(
            peripherals.i2c,
            peripherals.i2c_sda,
            peripherals.i2c_scl,
            &i2c_config,
        ).expect("Failed to create I2C sensor bus");
        
        Self {
            battery_channel,
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
    
    /// Update all sensor readings
    /// Call this periodically from the main loop
    pub fn update(&mut self) {
        let mut state = self.state.lock().unwrap();
        
        // Read battery voltage (with 0.5 voltage divider, so multiply by 2)
        if let Ok(raw) = self.battery_channel.read_raw() {
            // ADC with 12dB attenuation has ~0-3.3V range, 12-bit resolution
            // Voltage divider is 0.5, so actual battery voltage = reading * 2
            let voltage = (raw as f32 / 4095.0) * 3.3 * 2.0;
            state.battery_voltage = voltage;
        }

        //print battery voltage for debugging
        log::trace!("Battery voltage: {:.2} V", state.battery_voltage);
        
        // Read light sensor (enable first, then read)
        self.light_enable.set_high().ok();
        // Small delay would be ideal here, but for simplicity we read immediately
        if let Ok(raw) = self.light_channel.read_raw() {
            // Normalize to 0.0 - 1.0 range
            state.light_sensor = raw as f32 / 4095.0;
        }
        self.light_enable.set_low().ok();

        //print light sensor value for debugging
        log::trace!("Light sensor raw value: {}", state.light_sensor);

        
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
    pub fn apply_to_input(&self, input: &mut Input, current_time_ms: u32) {
        let state = self.state.lock().unwrap();
        
        // Update all sensors in the engine's input system
        input.update_sensor(SensorType::BatteryLevel, state.battery_voltage, current_time_ms);
        input.update_sensor(SensorType::Thermometer, state.thermometer, current_time_ms);
        input.update_sensor(SensorType::LightSensor, state.light_sensor, current_time_ms);
        input.update_sensor(SensorType::Accelerometer, state.accelerometer, current_time_ms);
        input.update_sensor(SensorType::MicLoudness, state.mic_loudness, current_time_ms);
    }
    
    /// Get current battery voltage (0.0 - 4.2V typical for Li-ion)
    pub fn get_battery_voltage(&self) -> f32 {
        self.state.lock().unwrap().battery_voltage
    }
    
    /// Get battery percentage (approximate, based on Li-ion discharge curve)
    pub fn get_battery_percentage(&self) -> u8 {
        let voltage = self.get_battery_voltage();
        // Simple linear approximation: 3.0V = 0%, 4.2V = 100%
        let percentage = ((voltage - 3.0) / 1.2 * 100.0).clamp(0.0, 100.0);
        percentage as u8
    }
    
    /// Get current light sensor reading (0.0 - 1.0)
    pub fn get_light_level(&self) -> f32 {
        self.state.lock().unwrap().light_sensor
    }
    
    /// Get current microphone level (0.0 - 1.0)
    pub fn get_mic_level(&self) -> f32 {
        self.state.lock().unwrap().mic_loudness
    }
    
    /// Get current temperature reading
    pub fn get_temperature(&self) -> f32 {
        self.state.lock().unwrap().thermometer
    }
}
