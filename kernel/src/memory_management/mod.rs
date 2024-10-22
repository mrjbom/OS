mod physical_memory_manager;

/// Inits Physical Memory Manager and Virtual Memory Manager
pub fn init(boot_info: &bootloader_api::BootInfo) {
    physical_memory_manager::init(boot_info);
}
