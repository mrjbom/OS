use crate::com_ports;
use spin::Mutex;

/// Serial port printer for QEMU logs writing
///
/// Locks COM1 PORT
///
/// **Don't use in interrupts**

pub static SERIAL_PRINTER: Mutex<SerialPrinter> = Mutex::new(SerialPrinter);

/// Serial port printer for QEMU logs writing
///
/// Locks COM1 PORT
///
/// **Don't use in interrupts**
pub struct SerialPrinter;

impl core::fmt::Write for SerialPrinter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut com1_port = com_ports::COM1_PORT.lock();
        for ch in s.bytes() {
            if !ch.is_ascii_control() || ch == b'\n' {
                com1_port.send(ch);
            }
        }
        Ok(())
    }
}

/// Prints ASCII string to COM1
///
/// Locks COM1 PORT
///
/// **Don't use in interrupts**
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        // Can't panic because used in panic handler
        let _ = $crate::serial_debug::serial_printer::SERIAL_PRINTER.lock().write_fmt(format_args!($($arg)*));
    });
}

/// Prints ASCII string with newline to COM1
///
/// Locks COM1 PORT
///
/// **Don't use in interrupts**
#[macro_export]
macro_rules! serial_println {
    () => (crate::serial_print!("\n"));
    ($($arg:tt)*) => (crate::serial_print!("{}\n", format_args!($($arg)*)));
}
