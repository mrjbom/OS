mod physical_memory_manager;

/// 4KB
const PAGE_SIZE: usize = 4096;

/// Inits Physical Memory Manager and Virtual Memory Manager
pub fn init(boot_info: &bootloader_api::BootInfo) {
    physical_memory_manager::init(boot_info);
}
