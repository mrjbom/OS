#![no_std]
#![no_main]

// TODO: Configure BootloaderConfig
bootloader_api::entry_point!(kmain);

#[no_mangle]
fn kmain(_boot_info: &'static mut bootloader_api::BootInfo)  -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
