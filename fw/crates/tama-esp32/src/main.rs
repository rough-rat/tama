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
use embedded_graphics::{
    prelude::*,
    pixelcolor::Rgb565,
    primitives::Rectangle,
};

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

    // Configure SPI pins
    let sclk = peripherals.pins.gpio12; // SPI Clock
    let sdo = peripherals.pins.gpio14;  // SPI MOSI (Data Out)
    let sdi = peripherals.pins.gpio0;   // SPI MISO (not used for display)
    let cs = peripherals.pins.gpio5;    // Chip Select

    // Display control pins
    let dc = peripherals.pins.gpio4;       // Data/Command
    let rst = peripherals.pins.gpio3;      // Reset
    let backlight = peripherals.pins.gpio7; // Backlight control

    log::info!("Configuring SPI with DMA...");

    // Configure SPI driver with DMA enabled for better performance
    // DMA allows large transfers without CPU intervention
    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        sclk,
        sdo,
        Some(sdi),
        &SpiDriverConfig::new().dma(Dma::Auto(32768)), // Enable DMA with 4KB buffer
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

    log::info!("Initializing display...");

    // Turn on backlight
    backlight_pin.set_high().unwrap();

    // Create display interface with heap-allocated buffer for better performance
    // Larger buffer = fewer SPI transactions = faster updates
    // Using heap allocation allows for much larger buffers without stack overflow
    // Full frame buffer would be 240*280*2 = 134,400 bytes
    let mut buffer = vec![0u8; 32768].into_boxed_slice(); // 64 KB buffer on heap
    let di = SpiInterface::new(spi_device, dc_pin, &mut *buffer);

    // Initialize the display
    // ST7789 240x280 display with 90-degree rotation
    // The physical panel is 240x280, controller supports 240x320
    // After 90-degree rotation: appears as 280x240 to the application
    let mut display = Builder::new(ST7789, di)
        .display_size(240, 280)  // Physical panel dimensions
        .display_offset(0, 20)   // ST7789 controller offset for 240x280 panels
        .orientation(Orientation::new().rotate(Rotation::Deg0))
        .invert_colors(ColorInversion::Inverted)
        .reset_pin(rst_pin)
        .init(&mut FreeRtos)
        .unwrap();

    log::info!("Display initialized successfully!");

    // // Test display with Hello World
    // use embedded_graphics::{
    //     mono_font::{ascii::FONT_10X20, MonoTextStyle},
    //     pixelcolor::Rgb565,
    //     prelude::*,
    //     primitives::{Circle, PrimitiveStyle, Rectangle},
    //     text::Text,
    // };

    // log::info!("Drawing Hello World test...");
    // display.clear(Rgb565::BLACK).unwrap();
    
    // // Draw a filled rectangle as background
    // Rectangle::new(Point::new(10, 10), Size::new(220, 80))
    //     .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
    //     .draw(&mut display)
    //     .unwrap();

    // // Draw text
    // let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    // Text::new("Hello, World!", Point::new(20, 40), text_style)
    //     .draw(&mut display)
    //     .unwrap();
    // Text::new("Display Test", Point::new(20, 65), text_style)
    //     .draw(&mut display)
    //     .unwrap();

    // // Draw a circle
    // Circle::new(Point::new(95, 120), 60)
    //     .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 3))
    //     .draw(&mut display)
    //     .unwrap();

    // log::info!("Hello World drawn, waiting 3 seconds...");
    // FreeRtos::delay_ms(3000);

    // Allocate framebuffer on heap for double buffering
    // 280x240 pixels * 2 bytes per pixel (RGB565) = 134,400 bytes
    log::info!("Allocating framebuffer (134,400 bytes)...");
    let mut framebuffer = Framebuffer::new(280, 240);
    log::info!("Framebuffer allocated successfully");

    // Initialize the game engine
    let mut engine = Engine::new();
    log::info!("Engine initialized");

    let mut frame_count = 0u32;
    
    // Main game loop
    loop {
        // Update game state
        log::info!("Engine start");
        engine.update();

        if frame_count % 30 == 0 {
            log::info!("Rendering frame {}...", frame_count);
        }
        
        // Render to framebuffer (fast - all in RAM)
        log::info!("Render start");

        if let Err(e) = engine.render(&mut framebuffer) {
            log::error!("Render error: {:?}", e);
        }
        
        // Transfer framebuffer to display (single DMA transfer)
        let bounding_box = Rectangle::new(Point::zero(), framebuffer.size());

        log::info!("Transfer start");
        if let Err(e) = display.fill_contiguous(&bounding_box, framebuffer.iter()) {
            log::error!("Display transfer error: {:?}", e);
        }
        log::info!("Transfer complete");
        
        frame_count = frame_count.wrapping_add(1);
        
        // Frame delay (~30 FPS)
        // FreeRtos::delay_ms(33);
    }
}
