mod physical_memory_manager;

/// 4KB
const PAGE_SIZE: u64 = 4096;

/// Inits Physical Memory Manager and Virtual Memory Manager
pub fn init(boot_info: &bootloader_api::BootInfo) {
    log::info!("Physical Memory Manager initialization");
    physical_memory_manager::init(boot_info);
}
