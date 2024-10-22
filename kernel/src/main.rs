#![no_std]
#![no_main]

use bootloader_api::config::Mapping;

mod com_ports;
mod gdt;
mod interrupts;
mod serial_debug;

static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.kernel_stack_size = 128 * 1024; // 128 KB

    // Configure mappings created by bootloader
    let mut mappings = bootloader_api::config::Mappings::new_default();
    // ../doc/virtual_memory_layout.txt
    mappings.dynamic_range_start = Some(0xFFFF_9000_0000_0000);
    mappings.dynamic_range_end = Some(0xFFFF_9FFF_FFFF_F000);
    // Complete physical memory mapping with offset
    mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_A000_0000_0000));

    config.mappings = mappings;

    config
};

bootloader_api::entry_point!(kmain, config = &BOOTLOADER_CONFIG);

#[no_mangle]
fn kmain(_boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    com_ports::init();
    serial_debug::serial_logger::init();
    log::info!("KERNEL START");

    // Init GDT
    serial_println!("GDT initialization");
    gdt::init();

    // Init and enable interrupts
    serial_println!("Interrupts initialization and enabling");
    interrupts::init();

    x86_64::instructions::interrupts::disable();
    log::info!("KERNEL FINISH");
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    serial_println_lock_free!("PANIC!!!");
    serial_println_lock_free!("{info}");
    loop {}
}
