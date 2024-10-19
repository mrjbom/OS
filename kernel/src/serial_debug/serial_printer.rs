use crate::com_ports;

/// Serial port printer for QEMU logs writing
///
/// Locks COM1 PORT
///
/// **Don't use in interrupts**
#[allow(dead_code)]
pub static mut SERIAL_PRINTER: SerialPrinter = SerialPrinter;

/// Serial port printer for QEMU logs writing but without locking COM1
///
/// Can be used in interrupts
#[allow(dead_code)]
pub static mut SERIAL_PRINTER_LOCK_FREE: SerialPrinterLockFree = SerialPrinterLockFree;

/// Serial port printer for QEMU logs writing
///
/// Locks COM1 PORT
///
/// **Don't use in interrupts**<br>
/// If an interrupt occurs during a locked COM1 port and the interrupt handler tries to use this function, it will freeze.
/// Use [SerialPrinterLockFree] in interrupts instead
pub struct SerialPrinter;

/// Serial port printer but not locks COM1 PORT
/// Useful for in interrupts printing
pub struct SerialPrinterLockFree;

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

impl core::fmt::Write for SerialPrinterLockFree {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.bytes() {
            if !ch.is_ascii_control() || ch == b'\n' {
                #[allow(static_mut_refs)]
                unsafe {
                    com_ports::COM1_PORT_LOCK_FREE.send(ch);
                }
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
        #[allow(static_mut_refs)]
        let result = unsafe { $crate::serial_debug::serial_printer::SERIAL_PRINTER.write_fmt(format_args!($($arg)*)) };
        result.expect("Failed to write to SERIAL_PRINTER")
    });
}

/// Prints ASCII string with newline to COM1
///
/// Locks COM1 PORT
///
/// **Don't use in interrupts**<br>
/// Use serial_println_lock_free instead
/// ```ignore
/// let mut com1_mutex_guard = com_ports::COM1_PORT.lock();
/// com1_mutex_guard.write_str("COM1 locked\n"); // Printed
/// //serial_println!("DEADLOCK!!!"); // Not printed, deadlock, infinite loop
/// serial_println_lock_free!("But serial_println_lock_free may print"); // Printed
/// ```
#[macro_export]
macro_rules! serial_println {
    () => (crate::serial_print!("\n"));
    ($($arg:tt)*) => (crate::serial_print!("{}\n", format_args!($($arg)*)));
}

// Lock free variants
// For interrupts printng

/// Prints ASCII string to COM1 without lock
///
/// Can be used in interrupts
#[macro_export]
macro_rules! serial_print_lock_free {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        // Can't panic because used in panic handler
        #[allow(static_mut_refs)]
        let _ = unsafe { $crate::serial_debug::serial_printer::SERIAL_PRINTER_LOCK_FREE.write_fmt(format_args!($($arg)*)) };
    });
}

/// Prints ASCII string to COM1 without lock
///
/// Can be used in interrupts
/// ```ignore
/// let mut com1_mutex_guard = com_ports::COM1_PORT.lock();
/// com1_mutex_guard.write_str("COM1 locked\n"); // Printed
/// //serial_println!("DEADLOCK!!!"); // Not printed, deadlock, infinite loop
/// serial_println_lock_free!("But serial_println_lock_free may print"); // Printed
/// ```
#[macro_export]
macro_rules! serial_println_lock_free {
    () => (crate::serial_print_lock_free!("\n"));
    ($($arg:tt)*) => (crate::serial_print_lock_free!("{}\n", format_args!($($arg)*)));
}
