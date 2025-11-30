//! I2C Sensor Drivers
//! 
//! This module provides drivers for I2C sensors on the shared I2C bus:
//! - MMA8451QR1 - 3-axis accelerometer (address: 0x1C or 0x1D)
//! - HDC1080DMBR - temperature and humidity sensor (address: 0x40)
//! 
//! Hardware connections:
//! - SDA: GPIO35
//! - SCL: GPIO36  
//! - ACC_INT1: GPIO47 (accelerometer interrupt)
//! - ACC_INT2: unconnected

use esp_idf_hal::gpio::{AnyIOPin, AnyInputPin};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::prelude::*;
use esp_idf_svc::sys::EspError;

use alloc::vec::Vec;

extern crate alloc;

/// Known I2C addresses for sensors on this board
pub mod addresses {
    /// MMA8451 accelerometer (SA0 pin determines address)
    pub const MMA8451_ADDR_SA0_LOW: u8 = 0x1C;
    pub const MMA8451_ADDR_SA0_HIGH: u8 = 0x1D;
    
    /// HDC1080 temperature/humidity sensor
    pub const HDC1080_ADDR: u8 = 0x40;
}

/// I2C bus configuration for the sensor rail
pub struct I2cBusConfig {
    pub baudrate: Hertz,
    pub timeout_ms: u32,
}

impl Default for I2cBusConfig {
    fn default() -> Self {
        Self {
            baudrate: 400.kHz().into(),
            timeout_ms: 1000,
        }
    }
}

/// Peripherals required for the I2C sensor bus
pub struct I2cSensorPeripherals<I2C> {
    pub i2c: I2C,
    pub sda: AnyIOPin,           // GPIO35
    pub scl: AnyIOPin,           // GPIO36
    pub acc_int1: AnyInputPin,   // GPIO47 - accelerometer interrupt 1
}

/// Trait for I2C sensor devices
/// 
/// All I2C sensors should implement this trait for consistent initialization
/// and basic operations.
pub trait I2cSensor {
    /// The I2C address of the device
    fn address(&self) -> u8;
    
    /// Human-readable name of the sensor
    fn name(&self) -> &'static str;
    
    /// Check if the sensor is present and responding
    fn is_present(&mut self) -> bool;
    
    /// Initialize the sensor with default configuration
    fn init(&mut self) -> Result<(), I2cSensorError>;
}

/// Common error type for I2C sensor operations
#[derive(Debug, Clone)]
pub enum I2cSensorError {
    /// I2C bus communication error
    BusError(i32),
    /// Device not found at expected address
    DeviceNotFound,
    /// Invalid data received from device
    InvalidData,
    /// Device reported an error condition
    DeviceError,
    /// Operation timed out
    Timeout,
    /// Sensor not initialized
    NotInitialized,
}

impl From<EspError> for I2cSensorError {
    fn from(e: EspError) -> Self {
        I2cSensorError::BusError(e.code())
    }
}

// ============================================================================
// I2C Scanner - Reference implementation and utility
// ============================================================================

/// I2C Bus Scanner
/// 
/// Scans the I2C bus for responding devices. This serves as:
/// 1. A reference implementation for the I2cSensor trait pattern
/// 2. A diagnostic tool for board bring-up
/// 3. A way to verify sensor presence before initialization
pub struct I2cScanner<'a, 'd> {
    i2c: &'a mut I2cDriver<'d>,
}

impl<'a, 'd> I2cScanner<'a, 'd> {
    /// Create a new I2C scanner
    pub fn new(i2c: &'a mut I2cDriver<'d>) -> Self {
        Self { i2c }
    }
    
    /// Scan the entire 7-bit I2C address range (0x08 - 0x77)
    /// 
    /// Returns a vector of addresses that responded to a probe.
    /// Excludes reserved addresses (0x00-0x07 and 0x78-0x7F).
    pub fn scan(&mut self) -> Vec<u8> {
        let mut found = Vec::new();
        
        // Scan valid 7-bit address range, excluding reserved addresses
        for addr in 0x08u8..=0x77u8 {
            if self.probe_address(addr) {
                found.push(addr);
            }
        }
        
        found
    }
    
    /// Probe a specific I2C address
    /// 
    /// Returns true if a device responded at the given address.
    pub fn probe_address(&mut self, addr: u8) -> bool {
        // Attempt to read 1 byte from the address
        // Most I2C devices will ACK even if we don't read valid data
        let mut buf = [0u8; 1];
        
        // Use a short timeout for scanning
        self.i2c.read(addr, &mut buf, 10).is_ok()
    }
    
    /// Scan the bus and generate a human-readable report
    /// 
    /// Returns a formatted string describing all found devices.
    pub fn scan_report(&mut self) -> String {
        let found = self.scan();
        
        if found.is_empty() {
            "I2C scan: No devices found".to_string()
        } else {
            let mut report = format!("I2C scan: Found {} device(s):", found.len());
            for addr in &found {
                let name = match *addr {
                    addresses::MMA8451_ADDR_SA0_LOW | 
                    addresses::MMA8451_ADDR_SA0_HIGH => "MMA8451 Accelerometer",
                    addresses::HDC1080_ADDR => "HDC1080 Temp/Humidity",
                    _ => "Unknown device",
                };
                report.push_str(&format!("\n[0x{:02X}: {}]", addr, name));
            }
            report
        }
    }
}

// ============================================================================
// Accelerometer Driver Stub (MMA8451QR1)
// ============================================================================

/// MMA8451 3-axis accelerometer driver
/// 
/// Features:
/// - 14-bit resolution
/// - ±2g/±4g/±8g selectable range
/// - Orientation detection
/// - Motion/freefall detection
/// - Interrupt support (INT1 on GPIO47)
pub struct Mma8451Driver<'a, 'd> {
    i2c: &'a mut I2cDriver<'d>,
    address: u8,
    initialized: bool,
}

impl<'a, 'd> Mma8451Driver<'a, 'd> {
    /// Create a new MMA8451 driver
    /// 
    /// The address depends on the SA0 pin:
    /// - SA0 = GND: 0x1C
    /// - SA0 = VCC: 0x1D
    pub fn new(i2c: &'a mut I2cDriver<'d>, address: u8) -> Self {
        Self {
            i2c,
            address,
            initialized: false,
        }
    }
    
    /// Create with default address (SA0 = LOW = 0x1C)
    pub fn new_default(i2c: &'a mut I2cDriver<'d>) -> Self {
        Self::new(i2c, addresses::MMA8451_ADDR_SA0_LOW)
    }
    
    /// Read the WHO_AM_I register to verify device identity
    pub fn read_who_am_i(&mut self) -> Result<u8, I2cSensorError> {
        const WHO_AM_I_REG: u8 = 0x0D;
        const EXPECTED_ID: u8 = 0x1A;
        
        let mut buf = [0u8; 1];
        self.i2c.write_read(self.address, &[WHO_AM_I_REG], &mut buf, 100)?;
        
        if buf[0] != EXPECTED_ID {
            log::warn!("MMA8451: Unexpected WHO_AM_I value: 0x{:02X} (expected 0x{:02X})", 
                      buf[0], EXPECTED_ID);
        }
        
        Ok(buf[0])
    }
    
    /// Read raw acceleration data (stub - returns zeros)
    pub fn read_acceleration_raw(&mut self) -> Result<(i16, i16, i16), I2cSensorError> {
        if !self.initialized {
            return Err(I2cSensorError::NotInitialized);
        }
        
        // TODO: Implement actual register reads
        // OUT_X_MSB (0x01), OUT_Y_MSB (0x03), OUT_Z_MSB (0x05)
        Ok((0, 0, 0))
    }
    
    /// Read acceleration in g units (stub - returns zeros)
    pub fn read_acceleration_g(&mut self) -> Result<(f32, f32, f32), I2cSensorError> {
        let (x, y, z) = self.read_acceleration_raw()?;
        
        // TODO: Apply proper scaling based on configured range
        // For ±2g range: divide by 4096 (14-bit, 4 counts per mg)
        let scale = 1.0 / 4096.0;
        
        Ok((x as f32 * scale, y as f32 * scale, z as f32 * scale))
    }
}

impl<'a, 'd> I2cSensor for Mma8451Driver<'a, 'd> {
    fn address(&self) -> u8 {
        self.address
    }
    
    fn name(&self) -> &'static str {
        "MMA8451 Accelerometer"
    }
    
    fn is_present(&mut self) -> bool {
        self.read_who_am_i().is_ok()
    }
    
    fn init(&mut self) -> Result<(), I2cSensorError> {
        // Verify device identity
        let id = self.read_who_am_i()?;
        if id != 0x1A {
            return Err(I2cSensorError::DeviceNotFound);
        }
        
        // TODO: Configure device
        // - Set to active mode
        // - Configure data rate
        // - Configure range (±2g default)
        // - Configure interrupts if needed
        
        log::info!("MMA8451: Initialized at address 0x{:02X}", self.address);
        self.initialized = true;
        Ok(())
    }
}

// ============================================================================
// Temperature/Humidity Driver Stub (HDC1080DMBR)
// ============================================================================

/// HDC1080 temperature and humidity sensor driver
/// 
/// Features:
/// - 14-bit temperature resolution
/// - 14-bit humidity resolution  
/// - Low power consumption
/// - Factory calibrated
pub struct Hdc1080Driver<'a, 'd> {
    i2c: &'a mut I2cDriver<'d>,
    address: u8,
    initialized: bool,
}

impl<'a, 'd> Hdc1080Driver<'a, 'd> {
    /// Create a new HDC1080 driver
    pub fn new(i2c: &'a mut I2cDriver<'d>) -> Self {
        Self {
            i2c,
            address: addresses::HDC1080_ADDR,
            initialized: false,
        }
    }
    
    /// Read the manufacturer ID register
    pub fn read_manufacturer_id(&mut self) -> Result<u16, I2cSensorError> {
        const MANUFACTURER_ID_REG: u8 = 0xFE;
        const EXPECTED_ID: u16 = 0x5449; // Texas Instruments
        
        let mut buf = [0u8; 2];
        self.i2c.write_read(self.address, &[MANUFACTURER_ID_REG], &mut buf, 100)?;
        
        let id = u16::from_be_bytes([buf[0], buf[1]]);
        
        if id != EXPECTED_ID {
            log::warn!("HDC1080: Unexpected Manufacturer ID: 0x{:04X} (expected 0x{:04X})", 
                      id, EXPECTED_ID);
        }
        
        Ok(id)
    }
    
    /// Read the device ID register
    pub fn read_device_id(&mut self) -> Result<u16, I2cSensorError> {
        const DEVICE_ID_REG: u8 = 0xFF;
        const EXPECTED_ID: u16 = 0x1050;
        
        let mut buf = [0u8; 2];
        self.i2c.write_read(self.address, &[DEVICE_ID_REG], &mut buf, 100)?;
        
        let id = u16::from_be_bytes([buf[0], buf[1]]);
        
        if id != EXPECTED_ID {
            log::warn!("HDC1080: Unexpected Device ID: 0x{:04X} (expected 0x{:04X})", 
                      id, EXPECTED_ID);
        }
        
        Ok(id)
    }
    
    /// Read temperature in degrees Celsius (stub)
    pub fn read_temperature(&mut self) -> Result<f32, I2cSensorError> {
        if !self.initialized {
            return Err(I2cSensorError::NotInitialized);
        }
        
        // TODO: Implement actual measurement
        // 1. Write to temperature register (0x00)
        // 2. Wait for conversion (typ 6.5ms for 14-bit)
        // 3. Read 2 bytes
        // 4. Convert: temp = (raw / 65536) * 165 - 40
        
        Ok(20.0) // Stub value
    }
    
    /// Read relative humidity in percent (stub)
    pub fn read_humidity(&mut self) -> Result<f32, I2cSensorError> {
        if !self.initialized {
            return Err(I2cSensorError::NotInitialized);
        }
        
        // TODO: Implement actual measurement
        // 1. Write to humidity register (0x01)
        // 2. Wait for conversion
        // 3. Read 2 bytes
        // 4. Convert: rh = (raw / 65536) * 100
        
        Ok(50.0) // Stub value
    }
    
    /// Read both temperature and humidity in a single operation (stub)
    pub fn read_temp_and_humidity(&mut self) -> Result<(f32, f32), I2cSensorError> {
        if !self.initialized {
            return Err(I2cSensorError::NotInitialized);
        }
        
        // TODO: Configure for combined measurement mode and read both
        
        Ok((20.0, 50.0)) // Stub values
    }
}

impl<'a, 'd> I2cSensor for Hdc1080Driver<'a, 'd> {
    fn address(&self) -> u8 {
        self.address
    }
    
    fn name(&self) -> &'static str {
        "HDC1080 Temp/Humidity"
    }
    
    fn is_present(&mut self) -> bool {
        self.read_manufacturer_id().is_ok()
    }
    
    fn init(&mut self) -> Result<(), I2cSensorError> {
        // Verify device identity
        let mfg_id = self.read_manufacturer_id()?;
        if mfg_id != 0x5449 {
            return Err(I2cSensorError::DeviceNotFound);
        }
        
        let dev_id = self.read_device_id()?;
        log::info!("HDC1080: Found device (Mfg: 0x{:04X}, Dev: 0x{:04X})", mfg_id, dev_id);
        
        // TODO: Configure device
        // - Set resolution (14-bit for both temp and humidity)
        // - Configure acquisition mode
        
        log::info!("HDC1080: Initialized at address 0x{:02X}", self.address);
        self.initialized = true;
        Ok(())
    }
}

// ============================================================================
// I2C Sensor Bus Manager
// ============================================================================

/// Manages the shared I2C bus for all sensors
/// 
/// This struct owns the I2C driver and provides methods to create
/// sensor drivers that borrow the bus.
pub struct I2cSensorBus<'d> {
    i2c: I2cDriver<'d>,
}

impl<'d> I2cSensorBus<'d> {
    /// Create a new I2C sensor bus
    pub fn new<I2C: esp_idf_hal::i2c::I2c>(
        i2c: impl Peripheral<P = I2C> + 'd,
        sda: AnyIOPin,
        scl: AnyIOPin,
        config: &I2cBusConfig,
    ) -> Result<Self, EspError> {
        let i2c_config = I2cConfig::new()
            .baudrate(config.baudrate);
        
        let i2c_driver = I2cDriver::new(i2c, sda, scl, &i2c_config)?;
        
        log::info!("I2C sensor bus initialized at {} Hz", config.baudrate.0);
        
        Ok(Self { i2c: i2c_driver })
    }
    
    /// Get a mutable reference to the I2C driver for use with sensors
    pub fn driver_mut(&mut self) -> &mut I2cDriver<'d> {
        &mut self.i2c
    }
    
    /// Scan the bus and return found addresses
    pub fn scan(&mut self) -> Vec<u8> {
        let mut scanner = I2cScanner::new(&mut self.i2c);
        scanner.scan()
    }
    
    /// Scan the bus and return a human-readable report
    pub fn scan_report(&mut self) -> String {
        let mut scanner = I2cScanner::new(&mut self.i2c);
        scanner.scan_report()
    }
    
    /// Check if a specific address responds
    pub fn probe(&mut self, address: u8) -> bool {
        let mut scanner = I2cScanner::new(&mut self.i2c);
        scanner.probe_address(address)
    }
}
