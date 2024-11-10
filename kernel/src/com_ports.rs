// COM ports global variables for synchronization

use spin::Mutex;

/// COM1 port for printing QEMU logs
///
/// **Don't use in interrupts**<br>
/// Use [COM1_PORT_LOCK_FREE] instead
pub static COM1_PORT: Mutex<uart_16550::SerialPort> =
    unsafe { Mutex::new(uart_16550::SerialPort::new(0x3F8)) };

/// Lock free COM1 port for printing QEMU logs in interrupts
pub static mut COM1_PORT_LOCK_FREE: uart_16550::SerialPort =
    unsafe { uart_16550::SerialPort::new(0x3F8) };

/// Inits COM ports
pub fn init() {
    #[allow(static_mut_refs)]
    unsafe {
        COM1_PORT_LOCK_FREE.init();
    };
}
