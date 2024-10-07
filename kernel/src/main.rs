#![no_std]
#![no_main]

mod com_ports;
mod serial_debug_printer;

static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.kernel_stack_size = 128 * 1024; // 128 KB

    // Configure mappings created by bootloader
    let mut mappings = bootloader_api::config::Mappings::new_default();
    // Higher half
    mappings.dynamic_range_start = Some(0xFFFF_8000_0000_0000);

    config.mappings = mappings;

    config
};

bootloader_api::entry_point!(kmain, config = &BOOTLOADER_CONFIG);

fn kmain(_boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    serial_debug_println!("Kernel loaded");
    serial_debug_println!("Kernel finish");
    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    serial_debug_print!("PANIC!!!");
    serial_debug_println!("{info}");
    loop {}
}
