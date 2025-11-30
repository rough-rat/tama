use esp_idf_hal::{
    delay::FreeRtos,
    gpio::PinDriver,
    ledc::{self, config::TimerConfig, LedcDriver, LedcTimerDriver},
    prelude::*,
    spi::{self, Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
};
use mipidsi::{
    interface::SpiInterface,
    models::ST7789,
    options::{ColorInversion, Orientation, Rotation},
    Builder,
};
use embedded_graphics::{
    prelude::*,
    pixelcolor::Rgb565,
    primitives::Rectangle,
};
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread::{self, JoinHandle};

use super::DisplaySpiPeripherals;

pub const DISPLAY_WIDTH: u32 = 240;
pub const DISPLAY_HEIGHT: u32 = 280;

// Simple framebuffer that implements DrawTarget
pub struct Framebuffer {
    data: Box<[Rgb565]>,
    width: u32,
    height: u32,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        let data = vec![Rgb565::BLACK; size].into_boxed_slice();
        Self { data, width, height }
    }
    
    pub fn iter(&self) -> impl Iterator<Item = Rgb565> + '_ {
        self.data.iter().copied()
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x >= 0 && point.x < self.width as i32 
                && point.y >= 0 && point.y < self.height as i32 {
                let index = (point.y as u32 * self.width + point.x as u32) as usize;
                self.data[index] = color;
            }
        }
        Ok(())
    }
}

/// Thread-safe framebuffer wrapper for IPC between cores
pub struct SharedFramebuffer {
    framebuffer: Arc<Mutex<Framebuffer>>,
    frame_ready: Arc<(Mutex<bool>, Condvar)>,
}

impl SharedFramebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            framebuffer: Arc::new(Mutex::new(Framebuffer::new(width, height))),
            frame_ready: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }
    
    fn clone_for_transfer(&self) -> (Arc<Mutex<Framebuffer>>, Arc<(Mutex<bool>, Condvar)>) {
        (Arc::clone(&self.framebuffer), Arc::clone(&self.frame_ready))
    }
    
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Framebuffer> {
        self.framebuffer.lock().unwrap()
    }
    
    pub fn signal_frame_ready(&self) {
        let (lock, cvar) = &*self.frame_ready;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
    }
}

/// Display driver that manages the ST7789 display in a separate thread.
/// 
/// The display is initialized in a dedicated transfer thread to allow
/// concurrent rendering on the main thread while display updates happen
/// in the background.
pub struct DisplayDriver {
    shared_fb: SharedFramebuffer,
    backlight_brightness: Arc<AtomicU8>,
    #[allow(dead_code)]
    transfer_thread: JoinHandle<()>,
}

impl DisplayDriver {
    /// Creates a new display driver from display peripherals.
    /// 
    /// This will:
    /// 1. Configure SPI with DMA
    /// 2. Set up PWM backlight (controlled via set_backlight())
    /// 3. Spawn a transfer thread that initializes the display and handles framebuffer transfers
    /// 
    /// Returns a DisplayDriver with a shared framebuffer that can be used
    /// for rendering from the main thread.
    pub fn new(
        display_peripherals: DisplaySpiPeripherals<spi::SPI2, ledc::CHANNEL0, ledc::TIMER0>,
    ) -> Self {
        // Display control pins
        let dc_pin = PinDriver::output(display_peripherals.control.dc).unwrap();
        let rst_pin = PinDriver::output(display_peripherals.control.rst).unwrap();

        log::info!("Configuring SPI with DMA...");

        // Configure SPI driver with DMA enabled for better performance
        let spi_driver = SpiDriver::new(
            display_peripherals.spi,
            display_peripherals.sclk,
            display_peripherals.sdo,
            Some(display_peripherals.sdi),
            &SpiDriverConfig::new().dma(Dma::Auto(32768)),
        )
        .unwrap();

        // Create SPI device with CS pin (80 MHz for maximum performance)
        let config = esp_idf_hal::spi::config::Config::new()
            .baudrate(80.MHz().into());
        
        let spi_device = SpiDeviceDriver::new(
            spi_driver,
            Some(display_peripherals.cs),
            &config,
        )
        .unwrap();

        log::info!("SPI with DMA configured successfully");

        log::info!("Preparing display hardware...");

        // Allocate shared framebuffer
        log::info!("Allocating shared framebuffer ({} bytes)...", 
            DISPLAY_WIDTH * DISPLAY_HEIGHT * 2);
        let shared_fb = SharedFramebuffer::new(DISPLAY_WIDTH, DISPLAY_HEIGHT);
        log::info!("Shared framebuffer allocated successfully");

        // Clone Arc references for the display transfer thread
        let (fb_arc, frame_ready_arc) = shared_fb.clone_for_transfer();

        // Shared backlight brightness (0-100%)
        let backlight_brightness = Arc::new(AtomicU8::new(100));
        let backlight_brightness_thread = Arc::clone(&backlight_brightness);

        // Extract backlight peripherals to move into thread
        let backlight_peripherals = display_peripherals.control.backlight;
        
        // Spawn display transfer thread
        log::info!("Spawning display transfer thread...");
        
        let transfer_thread = thread::Builder::new()
            .name("display_transfer".to_string())
            .stack_size(4096)
            .spawn(move || {
                log::info!("Display transfer thread started - initializing display...");

                // Initialize PWM backlight driver in the thread
                let timer_driver = LedcTimerDriver::new(
                    backlight_peripherals.timer,
                    &TimerConfig::new().frequency(25.kHz().into()),
                ).unwrap();
                
                let mut backlight_driver = LedcDriver::new(
                    backlight_peripherals.channel,
                    timer_driver,
                    backlight_peripherals.pin,
                ).unwrap();

                let max_duty = backlight_driver.get_max_duty();
                backlight_driver.set_duty(max_duty).unwrap(); // Start at 100%
                log::info!("PWM backlight initialized (max duty: {})", max_duty);

                let mut current_brightness: u8 = 100;
                
                // Create display interface with heap-allocated buffer
                let mut buffer = vec![0u8; 65535].into_boxed_slice();
                let di = SpiInterface::new(spi_device, dc_pin, &mut *buffer);

                // Initialize the display
                let mut display = Builder::new(ST7789, di)
                    .display_size(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16)
                    .display_offset(0, 20)
                    .orientation(Orientation::new().rotate(Rotation::Deg0))
                    .invert_colors(ColorInversion::Inverted)
                    .reset_pin(rst_pin)
                    .init(&mut FreeRtos)
                    .unwrap();

                log::info!("Display initialized successfully in transfer thread!");
                
                let (ready_lock, cvar) = &*frame_ready_arc;
                let mut frame_count = 0u32;
                
                loop {
                    // Wait for frame ready signal from main thread
                    let mut ready = ready_lock.lock().unwrap();
                    while !*ready {
                        ready = cvar.wait(ready).unwrap();
                    }
                    *ready = false;
                    drop(ready);

                    // Check if backlight brightness changed
                    let new_brightness = backlight_brightness_thread.load(Ordering::Relaxed);
                    if new_brightness != current_brightness {
                        let duty = (max_duty as u32 * new_brightness as u32 / 100) as u32;
                        backlight_driver.set_duty(duty).unwrap();
                        log::info!("Backlight changed: {}% (duty: {})", new_brightness, duty);
                        current_brightness = new_brightness;
                    }
                    
                    if frame_count % 120 == 0 {
                        log::info!("Transfer thread: Transferring frame {}...", frame_count);
                        
                        unsafe {
                            let current_task = esp_idf_svc::sys::xTaskGetCurrentTaskHandle();
                            let stack_high_water_mark = esp_idf_svc::sys::uxTaskGetStackHighWaterMark(current_task);
                            log::info!("Display thread stack high water mark: {} bytes remaining", stack_high_water_mark * 4);
                        }
                    }
                    
                    let lock_start = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
                    
                    let fb = fb_arc.lock().unwrap();
                    let lock_acquired = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
                    let bounding_box = Rectangle::new(Point::zero(), fb.size());
                    
                    log::trace!("Transfer thread: Transfer start");
                    let transfer_start = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
                    
                    if let Err(e) = display.fill_contiguous(&bounding_box, fb.iter()) {
                        log::error!("Transfer thread: Display transfer error: {:?}", e);
                    }
                    
                    let transfer_end = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
                    log::trace!("Transfer thread: Transfer complete");
                    
                    if frame_count % 30 == 0 {
                        let lock_wait_us = lock_acquired - lock_start;
                        let transfer_us = transfer_end - transfer_start;
                        let total_us = transfer_end - lock_start;
                        log::info!("Frame timing - Lock wait: {} us, Transfer: {} us ({} ms), Total: {} us ({} ms)", 
                            lock_wait_us, transfer_us, transfer_us / 1000, total_us, total_us / 1000);
                    }
                    
                    frame_count = frame_count.wrapping_add(1);
                }
            })
            .expect("Failed to spawn display transfer thread");

        Self {
            shared_fb,
            backlight_brightness,
            transfer_thread,
        }
    }

    /// Sets the backlight brightness (0-100%).
    /// The change will take effect on the next frame.
    pub fn set_backlight(&self, brightness: u8) {
        let brightness = brightness.min(100);
        self.backlight_brightness.store(brightness, Ordering::Relaxed);
    }

    /// Gets the current backlight brightness setting (0-100%).
    pub fn get_backlight(&self) -> u8 {
        self.backlight_brightness.load(Ordering::Relaxed)
    }

    /// Returns a reference to the shared framebuffer for rendering.
    pub fn framebuffer(&self) -> &SharedFramebuffer {
        &self.shared_fb
    }
}
