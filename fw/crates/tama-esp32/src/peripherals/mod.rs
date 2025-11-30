pub mod adc_bus;
mod button_driver;
mod display_driver;
mod power_control;
pub mod pwm_bus;
mod sensor_driver;
pub mod sensors_i2c;

pub use adc_bus::AdcBus;
pub use button_driver::ButtonDriver;
pub use display_driver::DisplayDriver;
pub use power_control::{PowerControl, PowerPeripherals};
pub use pwm_bus::{PwmBus, PwmPeripherals, BacklightControl};
pub use sensor_driver::SensorDriver;

use esp_idf_hal::adc;
use esp_idf_hal::gpio::{AnyInputPin, AnyIOPin, AnyOutputPin};
use esp_idf_hal::i2c::I2C0;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi;

pub struct SystemPeripherals<SPI> {
    pub adc1: adc::ADC1,
    pub buttons: ButtonPeripherals,
    pub display: DisplaySpiPeripherals<SPI>,
    pub power: PowerPeripherals,
    pub pwm: PwmPeripherals<
        esp_idf_hal::ledc::TIMER0,
        esp_idf_hal::ledc::CHANNEL0,
        esp_idf_hal::ledc::CHANNEL1,
        esp_idf_hal::gpio::Gpio48,
        esp_idf_hal::gpio::Gpio9,
    >,
    pub sensors: SensorPeripherals,
}

/// Button GPIO pins
/// Active low (directly connected to GND when pressed)
pub struct ButtonPeripherals {
    pub btn_a: AnyInputPin,    // GPIO15
    pub btn_b: AnyInputPin,    // GPIO7
    pub btn_up: AnyInputPin,   // GPIO8
    pub btn_down: AnyInputPin, // GPIO18
    pub btn_left: AnyInputPin, // GPIO17
    pub btn_right: AnyInputPin,// GPIO16
    pub btn_boot: AnyInputPin, // GPIO0 (BOOT button)
}

/// Sensor peripherals for light, mic, and I2C sensors
/// Note: Battery is handled by PowerPeripherals
pub struct SensorPeripherals {
    pub light_sensor_pin: esp_idf_hal::gpio::Gpio2, // GPIO2 - Light sensor
    pub light_sensor_enable: AnyOutputPin,          // GPIO40 - Light sensor enable
    pub mic_pin: esp_idf_hal::gpio::Gpio1,         // GPIO1 - Microphone
    // I2C sensor bus
    pub i2c: I2C0,
    pub i2c_sda: AnyIOPin,                         // GPIO35
    pub i2c_scl: AnyIOPin,                         // GPIO36
    pub acc_int1: AnyInputPin,                     // GPIO47 - accelerometer interrupt
}

pub struct DisplayControlPeripherals {
    pub dc: AnyOutputPin,
    pub rst: AnyOutputPin,
}

pub struct DisplaySpiPeripherals<SPI> {
    pub control: DisplayControlPeripherals,
    pub spi: SPI,
    pub sclk: AnyOutputPin,
    pub sdo: AnyOutputPin,
    pub sdi: AnyInputPin,
    pub cs: AnyOutputPin,
}

impl SystemPeripherals<spi::SPI2> {
    pub fn take() -> Self {
        let peripherals = Peripherals::take().unwrap();

        SystemPeripherals {
            adc1: peripherals.adc1,
            buttons: ButtonPeripherals {
                btn_a: peripherals.pins.gpio15.into(),
                btn_b: peripherals.pins.gpio7.into(),
                btn_up: peripherals.pins.gpio8.into(),
                btn_down: peripherals.pins.gpio18.into(),
                btn_left: peripherals.pins.gpio17.into(),
                btn_right: peripherals.pins.gpio16.into(),
                btn_boot: peripherals.pins.gpio0.into(),
            },
            power: PowerPeripherals {
                battery_pin: peripherals.pins.gpio4,
                peripheral_power_pin: peripherals.pins.gpio5.into(),
            },
            pwm: PwmPeripherals {
                timer: peripherals.ledc.timer0,
                backlight_channel: peripherals.ledc.channel0,
                backlight_pin: peripherals.pins.gpio48,
                buzzer_channel: peripherals.ledc.channel1,
                buzzer_pin: peripherals.pins.gpio9,
            },
            sensors: SensorPeripherals {
                light_sensor_pin: peripherals.pins.gpio2,
                light_sensor_enable: peripherals.pins.gpio40.into(),
                mic_pin: peripherals.pins.gpio1,
                // I2C sensor bus
                i2c: peripherals.i2c0,
                i2c_sda: peripherals.pins.gpio35.into(),
                i2c_scl: peripherals.pins.gpio36.into(),
                acc_int1: peripherals.pins.gpio47.into(),
            },
            display: DisplaySpiPeripherals {
                control: DisplayControlPeripherals {
                    dc: peripherals.pins.gpio41.into(),
                    rst: peripherals.pins.gpio39.into(),
                },
                spi: peripherals.spi2,
                sclk: peripherals.pins.gpio37.into(),
                sdo: peripherals.pins.gpio38.into(),
                sdi: peripherals.pins.gpio14.into(),
                cs: peripherals.pins.gpio42.into(),
            },
        }
    }
}
