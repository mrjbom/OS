// COM ports global variables for synchronization

use spin::Mutex;
use x86_64::instructions::port::Port;

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
        // Disable all COM1 interrupts (Input to qemu monitor triggers interrupts from COM1)
        // Disable DLAB (clear MSB in Line Control Register)
        let mut line_control_register = Port::<u8>::new(0x3F8 + 3);
        let mut line_control_register_value = line_control_register.read();
        line_control_register_value &= 0b0111_1111u8;
        line_control_register.write(line_control_register_value);

        // Disable all interrupts
        let mut interrupt_enable_register = Port::<u8>::new(0x3F8 + 1);
        interrupt_enable_register.write(0);
    };
}
