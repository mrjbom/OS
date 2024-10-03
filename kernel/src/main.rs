#![no_std]
#![no_main]

mod serial_printer;

// TODO: Configure BootloaderConfig
bootloader_api::entry_point!(kmain);

#[no_mangle]
fn kmain(_boot_info: &'static mut bootloader_api::BootInfo)  -> ! {
    serial_println!("Kernel started");
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
