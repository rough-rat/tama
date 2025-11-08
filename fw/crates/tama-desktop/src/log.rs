// idk, this doesn't work I can't compile on macOS when using any defmt macros

#[defmt::global_logger]
struct StdLogger;

unsafe impl defmt::Logger for StdLogger {
    fn acquire() {
        
    }

    unsafe fn flush() {

    }

    unsafe fn release() {

    }

    unsafe fn write(bytes: &[u8]) {
        println!("{}", String::from_utf8_lossy(bytes));
    }
}


