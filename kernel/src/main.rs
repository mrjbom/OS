#![no_std]
#![no_main]

use bootloader_api::config::Mapping;

mod com_ports;
mod serial_debug;

static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.kernel_stack_size = 128 * 1024; // 128 KB

    // Configure mappings created by bootloader
    let mut mappings = bootloader_api::config::Mappings::new_default();
    // Complete physical memory mapping with offset
    // ../doc/virtual_memory_layout.txt
    mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_9000_0000_0000));

    config.mappings = mappings;

    config
};

bootloader_api::entry_point!(kmain, config = &BOOTLOADER_CONFIG);

fn kmain(_boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    serial_debug::serial_logger::init();
    log::info!("KERNEL START");
    log::info!("KERNEL FINISH");
    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    serial_println!("PANIC!!!");
    serial_println!("{info}");
    loop {}
}
