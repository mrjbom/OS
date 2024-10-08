use log::{Record, Level, Metadata, LevelFilter};

#[allow(dead_code)]
static SERIAL_LOGGER: SerialLogger = SerialLogger;

struct SerialLogger;

impl log::Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            crate::serial_debug_println!("{}: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Inits logger
pub fn init() {
    log::set_logger(&SERIAL_LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info)).expect("Failed to init logger");
}
