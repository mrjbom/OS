// COM ports global variables for synchronization

use spin::Mutex;

lazy_static::lazy_static! {
    pub static ref COM1_PORT: Mutex<uart_16550::SerialPort> = {
        let com1_port = Mutex::new(unsafe { uart_16550::SerialPort::new(0x3F8) });
        com1_port.lock().init();
        com1_port
    };
}
