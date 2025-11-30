mod button_driver;
mod display_driver;
mod sensor_driver;

pub use button_driver::ButtonDriver;
pub use display_driver::DisplayDriver;
pub use sensor_driver::SensorDriver;

use esp_idf_hal::adc;
use esp_idf_hal::gpio::{AnyInputPin, AnyOutputPin};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi;

pub struct SystemPeripherals<SPI, BacklightChannel, BacklightTimer> {
    pub buttons: ButtonPeripherals,
    pub display: DisplaySpiPeripherals<SPI, BacklightChannel, BacklightTimer>,
    pub sensors: SensorPeripherals,
    pub gpio5: AnyOutputPin, // Power/enable pin that needs to be set high
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

/// Sensor peripherals for ADC and I2C sensors
pub struct SensorPeripherals {
    pub adc1: adc::ADC1,
    pub battery_pin: esp_idf_hal::gpio::Gpio4,     // GPIO4 - Battery voltage with 0.5 divider
    pub light_sensor_pin: esp_idf_hal::gpio::Gpio2, // GPIO2 - Light sensor
    pub light_sensor_enable: AnyOutputPin,          // GPIO40 - Light sensor enable
    pub mic_pin: esp_idf_hal::gpio::Gpio1,         // GPIO1 - Microphone
}

pub struct BacklightPeripherals<Channel, Timer> {
    pub pin: AnyOutputPin,
    pub channel: Channel,
    pub timer: Timer,
}

pub struct DisplayControlPeripherals<BacklightChannel, BacklightTimer> {
    pub backlight: BacklightPeripherals<BacklightChannel, BacklightTimer>,
    pub dc: AnyOutputPin,
    pub rst: AnyOutputPin,
}

pub struct DisplaySpiPeripherals<SPI, BacklightChannel, BacklightTimer> {
    pub control: DisplayControlPeripherals<BacklightChannel, BacklightTimer>,
    pub spi: SPI,
    pub sclk: AnyOutputPin,
    pub sdo: AnyOutputPin,
    pub sdi: AnyInputPin,
    pub cs: AnyOutputPin,
}

impl SystemPeripherals<spi::SPI2, esp_idf_hal::ledc::CHANNEL0, esp_idf_hal::ledc::TIMER0> {
    pub fn take() -> Self {
        let peripherals = Peripherals::take().unwrap();

        SystemPeripherals {
            buttons: ButtonPeripherals {
                btn_a: peripherals.pins.gpio15.into(),
                btn_b: peripherals.pins.gpio7.into(),
                btn_up: peripherals.pins.gpio8.into(),
                btn_down: peripherals.pins.gpio18.into(),
                btn_left: peripherals.pins.gpio17.into(),
                btn_right: peripherals.pins.gpio16.into(),
                btn_boot: peripherals.pins.gpio0.into(),
            },
            gpio5: peripherals.pins.gpio5.into(),
            sensors: SensorPeripherals {
                adc1: peripherals.adc1,
                battery_pin: peripherals.pins.gpio4,
                light_sensor_pin: peripherals.pins.gpio2,
                light_sensor_enable: peripherals.pins.gpio40.into(),
                mic_pin: peripherals.pins.gpio1,
            },
            display: DisplaySpiPeripherals {
                control: DisplayControlPeripherals {
                    backlight: BacklightPeripherals {
                        pin: peripherals.pins.gpio48.into(),
                        channel: peripherals.ledc.channel0,
                        timer: peripherals.ledc.timer0,
                    },
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
