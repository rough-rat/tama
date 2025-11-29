use esp_idf_hal::{
    delay::FreeRtos,
    gpio::PinDriver,
    prelude::*,
    spi::{SpiDeviceDriver, SpiDriver, SpiDriverConfig, Dma},
    units::FromValueType,
};
use mipidsi::{
    interface::SpiInterface,
    models::ST7789,
    options::{ColorInversion, Orientation, Rotation},
    Builder,
};
use tama_core::engine::Engine;
use tama_core::input::{Button, ButtonState};
use embedded_graphics::{
    prelude::*,
    pixelcolor::Rgb565,
    primitives::Rectangle,
};
use std::sync::{Arc, Mutex, Condvar};
use std::thread;

// Simple framebuffer that implements DrawTarget
struct Framebuffer {
    data: Box<[Rgb565]>,
    width: u32,
    height: u32,
}

impl Framebuffer {
    fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        let data = vec![Rgb565::BLACK; size].into_boxed_slice();
        Self { data, width, height }
    }
    
    fn iter(&self) -> impl Iterator<Item = Rgb565> + '_ {
        self.data.iter().copied()
    }
}

// Thread-safe framebuffer wrapper for IPC between cores
struct SharedFramebuffer {
    framebuffer: Arc<Mutex<Framebuffer>>,
    frame_ready: Arc<(Mutex<bool>, Condvar)>,
}

impl SharedFramebuffer {
    fn new(width: u32, height: u32) -> Self {
        Self {
            framebuffer: Arc::new(Mutex::new(Framebuffer::new(width, height))),
            frame_ready: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }
    
    fn clone_for_transfer(&self) -> (Arc<Mutex<Framebuffer>>, Arc<(Mutex<bool>, Condvar)>) {
        (Arc::clone(&self.framebuffer), Arc::clone(&self.frame_ready))
    }
    
    fn lock(&self) -> std::sync::MutexGuard<Framebuffer> {
        self.framebuffer.lock().unwrap()
    }
    
    fn signal_frame_ready(&self) {
        let (lock, cvar) = &*self.frame_ready;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
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

fn main() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Disable the task watchdog timer temporarily while debugging slow rendering
    // This prevents system resets during long-running operations
    unsafe {
        esp_idf_svc::sys::esp_task_wdt_deinit();
    }
    log::info!("Task watchdog timer disabled");

    log::info!("Tama ESP32 starting...");

    let peripherals = Peripherals::take().unwrap();

    // Configure button input (simple test - will be refactored later)
    let button_pin = peripherals.pins.gpio0;
    let button = PinDriver::input(button_pin).unwrap();
    log::info!("Button configured on GPIO0");

    // Set GPIO5 high before configuring SPI
    let mut gpio5 = PinDriver::output(peripherals.pins.gpio5).unwrap();
    gpio5.set_high().unwrap();
    log::info!("GPIO5 set high");

    // Configure SPI pins
    let sclk = peripherals.pins.gpio37; // SPI Clock
    let sdo = peripherals.pins.gpio38;  // SPI MOSI (Data Out)
    let sdi = peripherals.pins.gpio14;   // SPI MISO (not used for display)
    let cs = peripherals.pins.gpio42;    // Chip Select

    // Display control pins
    let dc = peripherals.pins.gpio41;       // Data/Command
    let rst = peripherals.pins.gpio39;      // Reset
    let backlight = peripherals.pins.gpio48; // Backlight control


    // // Configure SPI pins
    // let sclk = peripherals.pins.gpio12; // SPI Clock
    // let sdo = peripherals.pins.gpio14;  // SPI MOSI (Data Out)
    // let sdi = peripherals.pins.gpio48;   // SPI MISO (not used for display)
    // let cs = peripherals.pins.gpio5;    // Chip Select

    // // Display control pins
    // let dc = peripherals.pins.gpio4;       // Data/Command
    // let rst = peripherals.pins.gpio3;      // Reset
    // let backlight = peripherals.pins.gpio7; // Backlight control

    log::info!("Configuring SPI with DMA...");

    // Configure SPI driver with DMA enabled for better performance
    // DMA allows large transfers without CPU intervention
    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        sclk,
        sdo,
        Some(sdi),
        &SpiDriverConfig::new().dma(Dma::Auto(32768)), // Enable DMA
    )
    .unwrap();

    // Create SPI device with CS pin
    // ST7789 can handle up to 80 MHz, using 80 MHz for maximum performance
    let config = esp_idf_hal::spi::config::Config::new()
        .baudrate(80.MHz().into());
    
    let spi_device = SpiDeviceDriver::new(
        spi_driver,
        Some(cs),
        &config,
    )
    .unwrap();

    log::info!("SPI with DMA configured successfully");

    // Configure display control pins
    let dc_pin = PinDriver::output(dc).unwrap();
    let rst_pin = PinDriver::output(rst).unwrap();
    let mut backlight_pin = PinDriver::output(backlight).unwrap();

    log::info!("Preparing display hardware...");

    // Turn on backlight
    backlight_pin.set_high().unwrap();

    // Allocate framebuffer on heap for double buffering
    // 280x240 pixels * 2 bytes per pixel (RGB565) = 134,400 bytes
    log::info!("Allocating shared framebuffer (134,400 bytes)...");
    let shared_fb = SharedFramebuffer::new(240, 280);
    log::info!("Shared framebuffer allocated successfully");

    // Clone Arc references for the display transfer thread (Core 1)
    let (fb_arc, frame_ready_arc) = shared_fb.clone_for_transfer();
    
    // Spawn display transfer thread on Core 1
    log::info!("Spawning display transfer thread...");
    
    thread::Builder::new()
        .name("display_transfer".to_string())
        .stack_size(3092) // 16KB stack for display thread (needs space for display buffer)
        .spawn(move || {
            log::info!("Display transfer thread started - initializing display...");
            
            // Create display interface with heap-allocated buffer
            let mut buffer = vec![0u8; 65535].into_boxed_slice(); // 64 KB buffer on heap
            let di = SpiInterface::new(spi_device, dc_pin, &mut *buffer);

            // Initialize the display in this thread
            let mut display = Builder::new(ST7789, di)
                .display_size(240, 280)
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
                // Wait for frame ready signal from Core 0
                let mut ready = ready_lock.lock().unwrap();
                while !*ready {
                    ready = cvar.wait(ready).unwrap();
                }
                *ready = false;
                drop(ready); // Release lock before doing transfer
                
                if frame_count % 120 == 0 {
                    log::info!("Transfer thread: Transferring frame {}...", frame_count);
                    
                    // Check stack usage for display thread
                    unsafe {
                        let current_task = esp_idf_svc::sys::xTaskGetCurrentTaskHandle();
                        let stack_high_water_mark = esp_idf_svc::sys::uxTaskGetStackHighWaterMark(current_task);
                        log::info!("Display thread stack high water mark: {} bytes remaining", stack_high_water_mark * 4);
                    }
                }
                
                // Measure time to acquire lock and transfer
                let lock_start = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
                
                // Lock framebuffer and transfer to display
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
                
                // Log timing every 30 frames
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

    // Initialize the game engine
    let mut engine = Engine::new();
    log::info!("Engine initialized on Core 0");

    let mut frame_count = 0u32;
    let mut button_pressed = false; // Track button state for edge detection
    
    // Setup for constant FPS timing using vTaskDelayUntil
    const TARGET_FPS: u32 = 30;
    const FRAME_TIME_MS: u32 = 1000 / TARGET_FPS; // 33ms for 30 FPS
    let mut last_wake_time = unsafe { esp_idf_svc::sys::xTaskGetTickCount() };
    
    // Main game loop on Core 0 - Rendering only
    log::info!("Starting main game loop on Core 0 with target {} FPS...", TARGET_FPS);
    loop {
        // Simple button handling (will be refactored later)
        // GPIO0 is pulled high, button press pulls it low
        let button_is_low = button.is_low();
        
        if button_is_low && !button_pressed {
            // Button just pressed
            log::info!("Button A pressed");
            engine.input_mut().set_button(Button::A, ButtonState::JustPressed);
            engine.input_mut().set_button(Button::Up, ButtonState::JustPressed);

            button_pressed = true;
        } else if button_is_low && button_pressed {
            // Button held
            engine.input_mut().set_button(Button::A, ButtonState::Pressed);
            engine.input_mut().set_button(Button::Up, ButtonState::Pressed);
        } else if !button_is_low && button_pressed {
            // Button just released
            log::info!("Button A released");
            engine.input_mut().set_button(Button::A, ButtonState::JustReleased);
            engine.input_mut().set_button(Button::Up, ButtonState::JustReleased);

            button_pressed = false;
        } else {
            // Button not pressed
            engine.input_mut().set_button(Button::A, ButtonState::Released);
            engine.input_mut().set_button(Button::Up, ButtonState::Released);
        }
        
        // Update game state
        log::trace!("Core 0: Engine update");
        let update_start = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
        engine.update();
        let update_end = unsafe { esp_idf_svc::sys::esp_timer_get_time() };

        // Render to shared framebuffer (fast - all in RAM)
        log::trace!("Core 0: Render start");
        let render_start = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
        let lock_wait_start = render_start;
        
        {
            let mut fb = shared_fb.lock();
            let lock_acquired = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
            
            if let Err(e) = engine.render(&mut *fb) {
                log::error!("Core 0: Render error: {:?}", e);
            }
            
            let render_end = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
            
            // Log timing every 30 frames
            if frame_count % 30 == 0 {
                let update_us = update_end - update_start;
                let lock_wait_us = lock_acquired - lock_wait_start;
                let render_us = render_end - lock_acquired;
                let total_frame_us = render_end - update_start;
                
                log::info!("Core 0: Rendering frame {}...", frame_count);
                log::info!("Core 0 timing - Update: {} us, Lock wait: {} us, Render: {} us, Total: {} us ({} ms)", 
                    update_us, lock_wait_us, render_us, total_frame_us, total_frame_us / 1000);
                
                // Check stack usage for main thread
                unsafe {
                    let current_task = esp_idf_svc::sys::xTaskGetCurrentTaskHandle();
                    let stack_high_water_mark = esp_idf_svc::sys::uxTaskGetStackHighWaterMark(current_task);
                    log::info!("Main thread stack high water mark: {} bytes remaining", stack_high_water_mark * 4);
                }
            }
        } // Lock released here
        
        // Signal Core 1 that frame is ready for transfer
        log::trace!("Core 0: Signaling frame ready");
        shared_fb.signal_frame_ready();
        
        frame_count = frame_count.wrapping_add(1);
        
        // Constant FPS timing using vTaskDelayUntil
        // This ensures consistent frame timing regardless of execution time
        // NOTE: If display transfer on Core 1 is slower than rendering, the framebuffer
        // lock above will block Core 0 until transfer completes. This prevents tearing
        // but may cause frame drops if transfer takes longer than FRAME_TIME_MS.
        // To avoid this, consider double buffering or making the lock non-blocking.
        
        // Convert milliseconds to FreeRTOS ticks
        // FreeRTOS tick rate is typically 100 Hz (10ms per tick) or 1000 Hz (1ms per tick)
        // We use pdMS_TO_TICKS macro equivalent: (ms * configTICK_RATE_HZ) / 1000
        let ticks_to_wait = (FRAME_TIME_MS * esp_idf_svc::sys::configTICK_RATE_HZ) / 1000;
        
        unsafe {
            esp_idf_svc::sys::xTaskDelayUntil(
                &mut last_wake_time as *mut _,
                ticks_to_wait,
            );
        }
    }
}
