pub mod general_purpose_allocator;
pub mod physical_memory_manager;
pub mod slab_allocator;
pub mod virtual_memory_manager;

/// 4KB
pub const PAGE_SIZE: usize = 4096;

/// Inits Physical Memory Manager and Virtual Memory Manager
pub fn init(boot_info: &bootloader_api::BootInfo) {
    log::info!("Physical Memory Manager initialization");
    physical_memory_manager::init(boot_info);

    log::info!("Virtual Memory Manager initialization");
    virtual_memory_manager::init();

    log::info!("SLAB allocator initialization");
    slab_allocator::init();

    log::info!("General purpose allocator initialization");
    general_purpose_allocator::init();
}
