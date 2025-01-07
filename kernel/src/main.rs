#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![no_std]
#![no_main]
#![allow(unused, dead_code)]

use bootloader_api::config::Mapping;

mod acpi;
mod com_ports;
mod gdt;
mod interrupts;
mod memory_management;
mod serial_debug;
mod timers;

static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.kernel_stack_size = 128 * 1024; // 128 KB

    // Configure mappings created by bootloader
    let mut mappings = bootloader_api::config::Mappings::new_default();
    // doc/virtual_memory_layout.txt
    mappings.dynamic_range_start = Some(0xFFFF_9000_0000_0000);
    mappings.dynamic_range_end = Some(0xFFFF_9FFF_FFFF_F000);
    // Complete physical memory mapping with offset
    mappings.physical_memory = Some(Mapping::FixedAddress(0xFFFF_A000_0000_0000));

    config.mappings = mappings;

    config
};

bootloader_api::entry_point!(kmain, config = &BOOTLOADER_CONFIG);

#[no_mangle]
fn kmain(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    // Init COM ports and logger
    com_ports::init();
    serial_debug::serial_logger::init();

    // Kernel start
    log::info!("--- KERNEL START ---");

    // Init GDT
    log::info!("GDT initialization");
    gdt::init();

    // Fill IDT
    interrupts::idt::init();

    // Init memory manager
    log::info!("Memory Manager initialization");
    memory_management::init(boot_info);

    // Get ACPI tables
    log::info!("Getting ACPI tables");
    acpi::init(boot_info);

    // Init IO APIC, Bootstrap Processor Local APIC
    // But it doesn't enable interrupts
    log::info!("APIC interrupts initialization and enabling");
    interrupts::init();

    // Init timers
    log::info!("Timers initialization");
    timers::init();

    log::debug!("Sleep 3 seconds");
    timers::hpet::sleep(core::time::Duration::from_secs(3));

    for _ in 0..30 {
        let current_duration = timers::hpet::get_current_ticks_as_duration();
        log::debug!("{:?}", current_duration);
        timers::hpet::sleep(core::time::Duration::from_secs(1));
    }
    log::debug!("End {:?}", timers::hpet::get_current_ticks_as_duration());

    // MUST PRINT 32
    x86_64::instructions::interrupts::enable();
    log::debug!("Prints 32?");
    loop {}

    x86_64::instructions::interrupts::disable();
    // Kernel finish
    log::info!("--- KERNEL FINISH ---");
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    serial_println_lock_free!("PANIC!!!");
    serial_println_lock_free!("{info}");
    loop {
        x86_64::instructions::hlt();
    }
}
