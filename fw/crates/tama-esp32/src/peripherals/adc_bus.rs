//! ADC Bus - Shared ADC1 driver for multiple consumers
//!
//! This module provides a shared ADC driver that can be used by multiple
//! subsystems (PowerControl, SensorDriver, etc.) to create ADC channels.

use std::sync::Arc;

#[allow(deprecated)]
use esp_idf_hal::adc::attenuation::DB_11;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::adc::ADC1;
use esp_idf_hal::gpio::{self};

/// Shared ADC1 driver type
/// 
/// This is wrapped in Arc to allow multiple ADC channels to share
/// the same underlying driver.
pub type SharedAdc1Driver<'d> = Arc<AdcDriver<'d, ADC1>>;

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

/// ADC Bus manager
/// 
/// Owns the ADC1 peripheral and provides methods to create channels
/// that share the underlying driver.
pub struct AdcBus<'d> {
    driver: SharedAdc1Driver<'d>,
    config: AdcBusConfig,
}

impl<'d> AdcBus<'d> {
    /// Create a new ADC bus from the ADC1 peripheral
    pub fn new(adc1: ADC1, config: AdcBusConfig) -> Self {
        let driver = Arc::new(
            AdcDriver::new(adc1).expect("Failed to create ADC1 driver")
        );
        
        log::info!("ADC bus initialized");
        
        Self { driver, config }
    }
    
    /// Get a clone of the shared ADC driver
    /// 
    /// Use this to pass the driver to other modules that need to create
    /// their own ADC channels.
    pub fn shared_driver(&self) -> SharedAdc1Driver<'d> {
        self.driver.clone()
    }
    
    /// Get the default channel configuration
    #[allow(deprecated)]
    pub fn default_channel_config(&self) -> AdcChannelConfig {
        if self.config.use_11db_attenuation {
            AdcChannelConfig {
                attenuation: DB_11,
                ..Default::default()
            }
        } else {
            AdcChannelConfig::default()
        }
    }
    
    /// Create a channel for GPIO4 (battery voltage)
    pub fn create_battery_channel(&self, pin: gpio::Gpio4) 
        -> AdcChannelDriver<'d, gpio::Gpio4, SharedAdc1Driver<'d>> 
    {
        let config = self.default_channel_config();
        AdcChannelDriver::new(self.driver.clone(), pin, &config)
            .expect("Failed to create battery ADC channel")
    }
    
    /// Create a channel for GPIO2 (light sensor)
    pub fn create_light_channel(&self, pin: gpio::Gpio2)
        -> AdcChannelDriver<'d, gpio::Gpio2, SharedAdc1Driver<'d>>
    {
        let config = self.default_channel_config();
        AdcChannelDriver::new(self.driver.clone(), pin, &config)
            .expect("Failed to create light sensor ADC channel")
    }
    
    /// Create a channel for GPIO1 (microphone)
    pub fn create_mic_channel(&self, pin: gpio::Gpio1)
        -> AdcChannelDriver<'d, gpio::Gpio1, SharedAdc1Driver<'d>>
    {
        let config = self.default_channel_config();
        AdcChannelDriver::new(self.driver.clone(), pin, &config)
            .expect("Failed to create microphone ADC channel")
    }
}
