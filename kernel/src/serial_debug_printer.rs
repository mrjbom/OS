use crate::com_ports;
use spin::Mutex;

/// Serial port printer for QEMU logs writing
///
/// Locks [`SERIAL_DEBUG_PRINTER`]
///
/// **Don't use in interrupts**
#[allow(dead_code)]
pub static SERIAL_DEBUG_PRINTER: Mutex<SerialDebugPrinter> = Mutex::new(SerialDebugPrinter);

/// Serial port printer for QEMU logs writing
///
/// Locks [`SERIAL_DEBUG_PRINTER`]
///
/// **Don't use in interrupts**
pub struct SerialDebugPrinter;

impl core::fmt::Write for SerialDebugPrinter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.bytes() {
            if !ch.is_ascii_control() || ch == b'\n' {
                com_ports::COM1_PORT.lock().send(ch);
            }
        }
        Ok(())
    }
}

#[macro_export]
/// Prints ASCII string to COM1
///
/// Locks [`SERIAL_DEBUG_PRINTER`]
///
/// **Don't use in interrupts**
macro_rules! serial_debug_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = $crate::serial_debug_printer::SERIAL_DEBUG_PRINTER.lock().write_fmt(format_args!($($arg)*));
    });
}

#[macro_export]
/// Prints ASCII string with newline to COM1
///
/// Locks [`SERIAL_DEBUG_PRINTER`]
///
/// **Don't use in interrupts**
macro_rules! serial_debug_println {
    () => (serial_debug_print!("\n"));
    ($($arg:tt)*) => (serial_debug_print!("{}\n", format_args!($($arg)*)));
}
