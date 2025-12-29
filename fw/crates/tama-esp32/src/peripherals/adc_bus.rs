//! ADC Bus - Shared ADC1 driver for multiple consumers
//!
//! This module provides a shared ADC driver that can be used by multiple
//! subsystems (PowerControl, SensorDriver, etc.) to read ADC values.

use std::cell::UnsafeCell;

#[allow(deprecated)]
use esp_idf_hal::adc::attenuation::DB_11;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::adc::ADC1;
use esp_idf_hal::gpio::{self};

/// ADC Bus configuration
#[derive(Clone)]
pub struct AdcBusConfig {
    /// Use 11dB attenuation for ~0-3.3V input range
    pub use_11db_attenuation: bool,
}

impl Default for AdcBusConfig {
    fn default() -> Self {
        Self {
            use_11db_attenuation: true,
        }
    }
}

/// Pins required for ADC readings
pub struct AdcPins {
    /// Battery voltage pin (GPIO4)
    pub battery_pin: gpio::Gpio4,
    /// Light sensor pin (GPIO2)
    pub light_pin: gpio::Gpio2,
    /// Microphone pin (GPIO1)
    pub mic_pin: gpio::Gpio1,
}

/// Inner struct that holds everything in a heap-allocated, pinned location
struct AdcBusInner<'d> {
    driver: AdcDriver<'d, ADC1>,
    // UnsafeCell is needed because we create channels that borrow driver,
    // but we also need to call driver.read_raw() which needs &self
    battery_channel: UnsafeCell<Option<AdcChannelDriver<'d, gpio::Gpio4, &'d AdcDriver<'d, ADC1>>>>,
    light_channel: UnsafeCell<Option<AdcChannelDriver<'d, gpio::Gpio2, &'d AdcDriver<'d, ADC1>>>>,
    mic_channel: UnsafeCell<Option<AdcChannelDriver<'d, gpio::Gpio1, &'d AdcDriver<'d, ADC1>>>>,
}

/// ADC Bus manager
/// 
/// Owns the ADC1 peripheral and all ADC channels, providing
/// read methods for each sensor.
/// 
/// Uses Box to heap-allocate everything, ensuring stable addresses.
pub struct AdcBus<'d> {
    inner: Box<AdcBusInner<'d>>,
}

impl<'d> AdcBus<'d> {
    /// Create a new ADC bus from the ADC1 peripheral and pins
    #[allow(deprecated)]
    pub fn new(adc1: ADC1, pins: AdcPins, config: AdcBusConfig) -> Self {
        log::info!("Creating ADC1 driver...");
        let driver = AdcDriver::new(adc1).expect("Failed to create ADC1 driver");
        log::info!("ADC1 driver created successfully");
        
        let channel_config = if config.use_11db_attenuation {
            log::info!("Using 11dB attenuation for ADC channels");
            AdcChannelConfig {
                attenuation: DB_11,
                ..Default::default()
            }
        } else {
            log::info!("Using default attenuation for ADC channels");
            AdcChannelConfig::default()
        };
        
        // First, box the inner struct with driver but no channels yet
        let mut inner = Box::new(AdcBusInner {
            driver,
            battery_channel: UnsafeCell::new(None),
            light_channel: UnsafeCell::new(None),
            mic_channel: UnsafeCell::new(None),
        });
        
        // Now the driver has a stable heap address via the Box
        // We can safely create references to it
        let driver_ref: &'d AdcDriver<'d, ADC1> = unsafe {
            // SAFETY: The driver is heap-allocated via Box and won't move.
            // We're extending the lifetime to 'd which is valid because
            // the Box lives for the lifetime of AdcBus.
            &*(&inner.driver as *const AdcDriver<'d, ADC1>)
        };
        
        log::info!("Driver ptr: {:p}", driver_ref);
        
        // Create channels using the stable driver reference
        log::info!("Creating battery channel on GPIO4...");
        let battery_channel = AdcChannelDriver::new(
            driver_ref,
            pins.battery_pin, 
            &channel_config
        ).expect("Failed to create battery ADC channel");
        unsafe { *inner.battery_channel.get() = Some(battery_channel); }
        log::info!("Battery channel created");
        
        log::info!("Creating light channel on GPIO2...");
        let light_channel = AdcChannelDriver::new(
            driver_ref,
            pins.light_pin, 
            &channel_config
        ).expect("Failed to create light sensor ADC channel");
        unsafe { *inner.light_channel.get() = Some(light_channel); }
        log::info!("Light channel created");
        
        log::info!("Creating mic channel on GPIO1...");
        let mic_channel = AdcChannelDriver::new(
            driver_ref,
            pins.mic_pin, 
            &channel_config
        ).expect("Failed to create microphone ADC channel");
        unsafe { *inner.mic_channel.get() = Some(mic_channel); }
        log::info!("Mic channel created");
        
        log::info!("ADC bus initialized with all channels");
        
        Self { inner }
    }
    
    /// Read the battery voltage raw ADC value (0-4095)
    pub fn read_battery_raw(&mut self) -> u16 {
        let channel = unsafe { (*self.inner.battery_channel.get()).as_mut().unwrap() };
        match self.inner.driver.read_raw(channel) {
            Ok(val) => val,
            Err(e) => {
                log::error!("ADC battery read failed: {:?}", e);
                0
            }
        }
    }
    
    /// Read the light sensor raw ADC value (0-4095)
    pub fn read_light_raw(&mut self) -> u16 {
        let channel = unsafe { (*self.inner.light_channel.get()).as_mut().unwrap() };
        match self.inner.driver.read_raw(channel) {
            Ok(val) => val,
            Err(e) => {
                log::error!("ADC light read failed: {:?}", e);
                0
            }
        }
    }
    
    /// Read the microphone raw ADC value (0-4095)
    pub fn read_mic_raw(&mut self) -> u16 {
        let channel = unsafe { (*self.inner.mic_channel.get()).as_mut().unwrap() };
        match self.inner.driver.read_raw(channel) {
            Ok(val) => val,
            Err(e) => {
                log::error!("ADC mic read failed: {:?}", e);
                0
            }
        }
    }
}
