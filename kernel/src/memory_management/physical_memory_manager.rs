use super::PAGE_SIZE;
use spin::Mutex;
use x86_64::PhysAddr;

/// Buddy Allocator for DMA addresses (first 16 MB range), secondary allocator
/// May be used if main allocator don't have memory
/// First 1 MB reserved
///
/// Don't use before PMM initialization!
static DMA_MEMORY_BUDDY_ALLOCATOR: Mutex<buddy_alloc::BuddyAlloc> = Mutex::new(buddy_alloc::BuddyAlloc {
    buddy_ptr: core::ptr::null_mut::<buddy_alloc::buddy_alloc_sys::buddy>(),
});

/// Address of the first page of the DMA memory (first page of 2nd MB)
const DMA_MEMORY_FIRST_PAGE_ADDR: PhysAddr = PhysAddr::new(0x100000);
/// Address of the last page of the DMA memory (last page of 16th MB)
const DMA_MEMORY_LAST_PAGE_ADDR: PhysAddr = PhysAddr::new(0xFFF000);
/// DMA memory address space size (15 MB)
const DMA_MEMORY_ADDRESS_RANGE_SIZE: u64 = DMA_MEMORY_LAST_PAGE_ADDR.as_u64() + PAGE_SIZE - DMA_MEMORY_FIRST_PAGE_ADDR.as_u64();

/// Buddy Allocator for memory upper DMA zone (first 16 MB), main allocator
///
/// Don't use before PMM initialization!
static BUDDY_ALLOCATOR: Mutex<buddy_alloc::BuddyAlloc> = Mutex::new(buddy_alloc::BuddyAlloc {
    buddy_ptr: core::ptr::null_mut::<buddy_alloc::buddy_alloc_sys::buddy>(),
});

/// Inits Physical Memory Manager
pub fn init(boot_info: &bootloader_api::BootInfo) {
    // Init buddy allocators

    // Init DMA buddy allocator

    // Init main buddy allocator
}
