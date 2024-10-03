use x86_64::instructions::port::PortWriteOnly;
use spin::Mutex;

const COM1_PORT: u16 = 0x3F8;

/// Serial port printer for QEMU log writing
///
/// Don't use in interrupts
#[allow(dead_code)]
pub static SERIAL_PRINTER: Mutex<SerialPrinter> = Mutex::new(SerialPrinter);

pub struct SerialPrinter;

// TODO: Move COM1 to global variable for port access synchronization
impl core::fmt::Write for SerialPrinter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // COM1: 0x3F8
        let mut com1_port = PortWriteOnly::new(COM1_PORT);
        for ch in s.bytes() {
            if !ch.is_ascii_control() || ch == b'\n' {
                unsafe { com1_port.write(ch); }
            }
        }
        Ok(())
    }
}

#[macro_export]
/// Prints to COM1
///
/// Locks SERIAL_PRINTER
/// Don't use in interrupts
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = $crate::serial_printer::SERIAL_PRINTER.lock().write_fmt(format_args!($($arg)*));
    });
}

#[macro_export]
/// Prints to COM1
///
/// Locks SERIAL_PRINTER
/// Don't use in interrupts
macro_rules! serial_println {
    () => (serial_print!("\n"));
    ($($arg:tt)*) => (serial_print!("{}\n", format_args!($($arg)*)));
}
