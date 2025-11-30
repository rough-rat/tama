//! PWM Bus - Shared LEDC timer for multiple PWM channels
//!
//! This module provides a shared LEDC timer that can be used by multiple
//! subsystems (display backlight, buzzer) to avoid clock source conflicts.
//! All channels share the same timer configuration (resolution, clock source).
//!
//! A dedicated PWM thread monitors the control interfaces and updates the
//! hardware accordingly.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::thread::{self, JoinHandle};

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::OutputPin;
use esp_idf_hal::ledc::{
    config::TimerConfig, LedcChannel, LedcDriver, LedcTimerDriver, Resolution,
};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::prelude::*;

use tama_core::buzzer::BuzzerTrait;

/// PWM Bus peripherals - raw LEDC channels and timer
pub struct PwmPeripherals<T, C0, C1, P0, P1> {
    pub timer: T,
    pub backlight_channel: C0,
    pub backlight_pin: P0,
    pub buzzer_channel: C1,
    pub buzzer_pin: P1,
}

/// Shared backlight control interface
/// 
/// Thread-safe wrapper for controlling backlight brightness.
/// The PWM worker thread monitors this and updates hardware.
#[derive(Clone)]
pub struct BacklightControl {
    brightness: Arc<AtomicU8>,
    max_duty: Arc<AtomicU32>,
}

impl BacklightControl {
    /// Set backlight brightness (0-100%)
    pub fn set_brightness(&self, percent: u8) {
        self.brightness.store(percent.min(100), Ordering::Relaxed);
    }
    
    /// Get current brightness setting (0-100%)
    pub fn get_brightness(&self) -> u8 {
        self.brightness.load(Ordering::Relaxed)
    }
}

/// Shared buzzer control interface
/// 
/// Thread-safe wrapper for controlling buzzer beeps.
/// The PWM worker thread monitors this and plays tones.
#[derive(Clone)]
pub struct BuzzerControl {
    frequency: Arc<AtomicU32>,
    duration_ms: Arc<AtomicU32>,
    max_duty: Arc<AtomicU32>,
}

impl BuzzerControl {
    /// Check if there's a pending beep command and consume it
    /// Returns Some((frequency, duration)) if there's a command
    fn take_command(&self) -> Option<(u32, u32)> {
        let duration = self.duration_ms.swap(0, Ordering::Acquire);
        if duration > 0 {
            let freq = self.frequency.load(Ordering::Relaxed);
            Some((freq, duration))
        } else {
            None
        }
    }
}

impl BuzzerTrait for BuzzerControl {
    fn beep(&self, frequency_hz: u32, duration_ms: u32) {
        self.frequency.store(frequency_hz, Ordering::Relaxed);
        self.duration_ms.store(duration_ms, Ordering::Release);
    }
}

/// PWM Bus manager
/// 
/// Owns the LEDC timer and creates channels for backlight and buzzer.
/// Both channels share the same timer to avoid clock source conflicts.
/// A dedicated thread monitors the control interfaces and updates hardware.
pub struct PwmBus {
    backlight_control: BacklightControl,
    buzzer_control: BuzzerControl,
    #[allow(dead_code)]
    worker_thread: JoinHandle<()>,
}

impl PwmBus {
    /// Create a new PWM bus from LEDC peripherals
    /// 
    /// Initializes a shared timer with settings compatible for both:
    /// - Backlight: PWM dimming at base frequency
    /// - Buzzer: Variable frequency beeps
    /// 
    /// Spawns a worker thread that monitors control interfaces and updates hardware.
    pub fn new<T, C0, C1, P0, P1>(
        peripherals: PwmPeripherals<T, C0, C1, P0, P1>,
    ) -> Self
    where
        T: esp_idf_hal::ledc::LedcTimer<SpeedMode = esp_idf_hal::ledc::LowSpeed> + Send + 'static,
        T: Peripheral<P = T>,
        C0: LedcChannel<SpeedMode = esp_idf_hal::ledc::LowSpeed> + Send + 'static,
        C0: Peripheral<P = C0>,
        C1: LedcChannel<SpeedMode = esp_idf_hal::ledc::LowSpeed> + Send + 'static,
        C1: Peripheral<P = C1>,
        P0: OutputPin + Send + 'static,
        P1: OutputPin + Send + 'static,
    {
        log::info!("Initializing PWM bus...");
        
        // Create shared control interfaces
        let backlight_control = BacklightControl {
            brightness: Arc::new(AtomicU8::new(100)), // Start at 100%
            max_duty: Arc::new(AtomicU32::new(0)),    // Will be set by worker thread
        };
        
        let buzzer_control = BuzzerControl {
            frequency: Arc::new(AtomicU32::new(0)),
            duration_ms: Arc::new(AtomicU32::new(0)),
            max_duty: Arc::new(AtomicU32::new(0)),    // Will be set by worker thread
        };
        
        // Clone controls for the worker thread
        let backlight_thread = backlight_control.clone();
        let buzzer_thread = buzzer_control.clone();
        
        // Spawn worker thread that owns the actual PWM drivers
        let worker_thread = thread::Builder::new()
            .name("pwm_bus".to_string())
            .stack_size(4096)
            .spawn(move || {
                pwm_worker_thread(peripherals, backlight_thread, buzzer_thread);
            })
            .expect("Failed to spawn PWM bus worker thread");
        
        log::info!("PWM bus initialized");
        
        Self {
            backlight_control,
            buzzer_control,
            worker_thread,
        }
    }
    
    /// Get a clone of the backlight control interface
    pub fn backlight(&self) -> BacklightControl {
        self.backlight_control.clone()
    }
    
    /// Get a clone of the buzzer control interface  
    pub fn buzzer(&self) -> BuzzerControl {
        self.buzzer_control.clone()
    }
}

/// PWM worker thread - owns the LEDC drivers and updates hardware based on control interfaces
fn pwm_worker_thread<T, C0, C1, P0, P1>(
    peripherals: PwmPeripherals<T, C0, C1, P0, P1>,
    backlight: BacklightControl,
    buzzer: BuzzerControl,
)
where
    T: esp_idf_hal::ledc::LedcTimer<SpeedMode = esp_idf_hal::ledc::LowSpeed>,
    T: Peripheral<P = T>,
    C0: LedcChannel<SpeedMode = esp_idf_hal::ledc::LowSpeed>,
    C0: Peripheral<P = C0>,
    C1: LedcChannel<SpeedMode = esp_idf_hal::ledc::LowSpeed>,
    C1: Peripheral<P = C1>,
    P0: OutputPin,
    P1: OutputPin,
{
    log::info!("PWM worker thread started");
    
    // Initialize timer with 10-bit resolution at 25kHz for flicker-free backlight
    let mut timer_driver = LedcTimerDriver::new(
        peripherals.timer,
        &TimerConfig::new()
            .frequency(25.kHz().into())
            .resolution(Resolution::Bits10),
    ).expect("Failed to initialize PWM timer");
    
    log::info!("PWM timer initialized (25kHz, 10-bit)");
    
    // Create backlight channel
    let mut backlight_driver = LedcDriver::new(
        peripherals.backlight_channel,
        &timer_driver,
        peripherals.backlight_pin,
    ).expect("Failed to initialize backlight PWM");
    
    let backlight_max_duty = backlight_driver.get_max_duty();
    backlight.max_duty.store(backlight_max_duty, Ordering::Relaxed);
    log::info!("Backlight PWM initialized (max duty: {})", backlight_max_duty);
    
    // Set initial backlight to 100%
    backlight_driver.set_duty(backlight_max_duty).unwrap();
    
    // Create buzzer channel  
    let mut buzzer_driver = LedcDriver::new(
        peripherals.buzzer_channel,
        &timer_driver,
        peripherals.buzzer_pin,
    ).expect("Failed to initialize buzzer PWM");
    
    let buzzer_max_duty = buzzer_driver.get_max_duty();
    buzzer.max_duty.store(buzzer_max_duty, Ordering::Relaxed);
    log::info!("Buzzer PWM initialized (max duty: {})", buzzer_max_duty);
    
    // Buzzer starts silent
    buzzer_driver.set_duty(0).unwrap();
    
    let mut current_brightness: u8 = 100;
    let mut buzzer_end_time: Option<i64> = None;
    
    loop {
        // Check backlight brightness changes
        let new_brightness = backlight.brightness.load(Ordering::Relaxed);
        if new_brightness != current_brightness {
            let duty = (backlight_max_duty as u32 * new_brightness as u32 / 100) as u32;
            backlight_driver.set_duty(duty).unwrap();
            log::info!("Backlight: {}% (duty: {})", new_brightness, duty);
            current_brightness = new_brightness;
        }
        
        // Check for new buzzer command
        if let Some((freq, duration)) = buzzer.take_command() {
            log::info!("Buzzer: {}Hz for {}ms", freq, duration);
            
            if freq >= 200 && freq <= 20000 {
                // Change timer frequency to match buzzer tone
                // This temporarily affects backlight too, but short beeps should be fine
                timer_driver.set_frequency(Hertz(freq)).ok();
                
                // Set 50% duty for square wave on buzzer
                let duty = buzzer_max_duty / 2;
                buzzer_driver.set_duty(duty).unwrap();
                
                // Calculate end time
                let now = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
                buzzer_end_time = Some(now + (duration as i64 * 1000));
            }
        }
        
        // Check if buzzer should stop
        if let Some(end_time) = buzzer_end_time {
            let now = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
            if now >= end_time {
                buzzer_driver.set_duty(0).unwrap();
                buzzer_end_time = None;
                
                // Restore backlight frequency
                timer_driver.set_frequency(25.kHz().into()).ok();
            }
        }
        
        // Small delay to avoid busy-waiting
        FreeRtos::delay_ms(5);
    }
}
