use log::{LevelFilter, Metadata, Record};

#[allow(dead_code)]
static SERIAL_LOGGER: SerialLogger = SerialLogger;

struct SerialLogger;

impl log::Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            crate::serial_println!("{}: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Inits logger
pub fn init() {
    log::set_logger(&SERIAL_LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .expect("Failed to init logger");
}
