//! Power Control - Battery monitoring, peripheral power, and sleep management
//!
//! This module handles:
//! - Battery voltage monitoring via ADC
//! - Peripheral power control via GPIO5 (load switch)
//! - Charging state readout (future: dedicated GPIOs)
//! - Sleep/deep sleep state management (future)
//! - Critical battery error handling (future)

use std::sync::{Arc, Mutex};

use esp_idf_hal::adc::oneshot::AdcChannelDriver;
use esp_idf_hal::gpio::{self, AnyOutputPin, Output, PinDriver};

use crate::peripherals::adc_bus::{AdcBus, SharedAdc1Driver};

/// Power state information
#[derive(Default, Clone, Debug)]
pub struct PowerState {
    /// Battery voltage in volts (0.0 - 4.2V for Li-ion)
    pub battery_voltage: f32,
    /// Battery percentage (0 - 100)
    pub battery_percentage: u8,
    /// Whether peripheral power is enabled
    pub peripheral_power_enabled: bool,
    /// Whether device is currently charging (future)
    pub is_charging: bool,
    /// Whether charger is connected (future)
    pub charger_connected: bool,
}

type SharedPowerState = Arc<Mutex<PowerState>>;

/// Peripherals required for power control
pub struct PowerPeripherals {
    /// Battery voltage ADC pin (GPIO4) with 0.5 voltage divider
    pub battery_pin: gpio::Gpio4,
    /// Peripheral power enable pin (GPIO5) - controls load switch
    pub peripheral_power_pin: AnyOutputPin,
    // Future: charging status GPIO
    // Future: charger connected GPIO
}

/// Power controller
/// 
/// Manages battery monitoring, peripheral power, and power states.
pub struct PowerControl<'d> {
    /// Battery voltage ADC channel
    battery_channel: AdcChannelDriver<'d, gpio::Gpio4, SharedAdc1Driver<'d>>,
    
    /// Peripheral power control pin (GPIO5)
    peripheral_power: PinDriver<'d, AnyOutputPin, Output>,
    
    /// Shared power state
    state: SharedPowerState,
}

impl<'d> PowerControl<'d> {
    /// Create a new power controller
    /// 
    /// Peripheral power starts DISABLED - call `set_peripheral_power(true)` 
    /// to enable peripherals after initialization.
    pub fn new(adc_bus: &AdcBus<'d>, peripherals: PowerPeripherals) -> Self {
        // Create battery ADC channel
        let battery_channel = adc_bus.create_battery_channel(peripherals.battery_pin);
        
        // Initialize peripheral power pin - start LOW (disabled)
        let mut peripheral_power = PinDriver::output(peripherals.peripheral_power_pin)
            .expect("Failed to create peripheral power pin");
        peripheral_power.set_low().ok();
        
        log::info!("Power control initialized (peripheral power OFF)");
        
        Self {
            battery_channel,
            peripheral_power,
            state: Arc::new(Mutex::new(PowerState::default())),
        }
    }
    
    /// Enable or disable peripheral power (GPIO5 load switch)
    /// 
    /// This controls power to external peripherals like the display.
    /// Should be enabled early in initialization, before accessing
    /// powered peripherals.
    pub fn set_peripheral_power(&mut self, enabled: bool) {
        if enabled {
            self.peripheral_power.set_high().ok();
        } else {
            self.peripheral_power.set_low().ok();
        }
        
        if let Ok(mut state) = self.state.lock() {
            state.peripheral_power_enabled = enabled;
        }
        
        log::info!("Peripheral power: {}", if enabled { "ON" } else { "OFF" });
    }
    
    /// Check if peripheral power is enabled
    pub fn is_peripheral_power_enabled(&self) -> bool {
        self.state.lock()
            .map(|s| s.peripheral_power_enabled)
            .unwrap_or(false)
    }
    
    /// Update battery readings
    /// 
    /// Call this periodically to update battery voltage and percentage.
    pub fn update(&mut self) {
        if let Ok(raw) = self.battery_channel.read_raw() {
            // ADC with 11dB attenuation has ~0-3.3V range, 12-bit resolution
            // Voltage divider is 0.5, so actual battery voltage = reading * 2
            let voltage = (raw as f32 / 4095.0) * 3.3 * 2.0;
            
            // Calculate percentage (simple linear approximation)
            // 3.0V = 0%, 4.2V = 100%
            let percentage = ((voltage - 3.0) / 1.2 * 100.0).clamp(0.0, 100.0) as u8;
            
            if let Ok(mut state) = self.state.lock() {
                state.battery_voltage = voltage;
                state.battery_percentage = percentage;
            }
            
            log::trace!("Battery: {:.2}V ({}%)", voltage, percentage);
        }
    }
    
    /// Get current battery voltage (0.0 - 4.2V typical for Li-ion)
    pub fn get_battery_voltage(&self) -> f32 {
        self.state.lock()
            .map(|s| s.battery_voltage)
            .unwrap_or(0.0)
    }
    
    /// Get battery percentage (0 - 100)
    pub fn get_battery_percentage(&self) -> u8 {
        self.state.lock()
            .map(|s| s.battery_percentage)
            .unwrap_or(0)
    }
    
    /// Get a clone of the shared power state
    pub fn shared_state(&self) -> SharedPowerState {
        self.state.clone()
    }
    
    /// Check if battery is critically low (< 5%)
    pub fn is_battery_critical(&self) -> bool {
        self.get_battery_percentage() < 5
    }
    
    /// Check if battery is low (< 20%)
    pub fn is_battery_low(&self) -> bool {
        self.get_battery_percentage() < 20
    }
    
    // Future methods:
    // pub fn is_charging(&self) -> bool { ... }
    // pub fn is_charger_connected(&self) -> bool { ... }
    // pub fn enter_sleep(&mut self) { ... }
    // pub fn enter_deep_sleep(&mut self) { ... }
}
