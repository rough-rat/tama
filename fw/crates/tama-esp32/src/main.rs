use esp_idf_hal::gpio::PinDriver;
use tama_core::engine::Engine;

mod peripherals;

use peripherals::{ButtonDriver, DisplayDriver, SensorDriver, SystemPeripherals};

use tama_core::input::SensorType;


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

    let peripherals = SystemPeripherals::take();

    // Initialize button driver
    let mut button_driver = ButtonDriver::new(peripherals.buttons);
    log::info!("Button driver configured");

    // Initialize sensor driver
    let mut sensor_driver = SensorDriver::new(peripherals.sensors);
    log::info!("Sensor driver configured");

    // Scan I2C bus for connected sensors
    log::info!("{}", sensor_driver.scan_i2c_rail_report());

    // Set GPIO5 high before configuring SPI
    let mut gpio5 = PinDriver::output(peripherals.gpio5).unwrap();
    gpio5.set_high().unwrap();
    log::info!("GPIO5 set high");

    // Initialize display driver (spawns transfer thread internally)
    let display_driver = DisplayDriver::new(peripherals.display);

    display_driver.set_backlight(10);

    // Initialize the game engine
    let mut engine = Engine::new();
    log::info!("Engine initialized on Core 0");

    let mut frame_count = 0u32;
    
    // Setup for constant FPS timing using vTaskDelayUntil
    const TARGET_FPS: u32 = 30;
    const FRAME_TIME_MS: u32 = 1000 / TARGET_FPS; // 33ms for 30 FPS
    let mut last_wake_time = unsafe { esp_idf_svc::sys::xTaskGetTickCount() };
    
    // Main game loop on Core 0 - Rendering only
    log::info!("Starting main game loop on Core 0 with target {} FPS...", TARGET_FPS);
    loop {
        // Update button states from GPIO
        button_driver.update();
        button_driver.apply_to_input(engine.input_mut());
        
        // Update sensor readings
        sensor_driver.update();
        let current_time_ms = (unsafe { esp_idf_svc::sys::esp_timer_get_time() } / 1000) as u32;
        sensor_driver.apply_to_input(engine.input_mut(), current_time_ms);
        
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
            let mut fb = display_driver.framebuffer().lock();
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

                // print battery level and light sensor status
                let battery_voltage = engine.input_mut().get_sensor_value(SensorType::BatteryLevel);
                let light_sensor = engine.input_mut().get_sensor_value(SensorType::LightSensor);
                log::info!("Battery voltage: {:.2} V, Light sensor: {:.2}%", battery_voltage, light_sensor * 100.0);
            }
        } // Lock released here
        
        // Signal Core 1 that frame is ready for transfer
        log::trace!("Core 0: Signaling frame ready");
        display_driver.framebuffer().signal_frame_ready();
        
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
