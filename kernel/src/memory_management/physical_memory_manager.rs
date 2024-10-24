use super::PAGE_SIZE;
use spin::Mutex;
use x86_64::PhysAddr;
use buddy_alloc::BuddyAlloc;
use crate::serial_println;

/// Buddy Allocator for DMA addresses (first 16 MB range), secondary allocator
/// May be used if main allocator don't have memory
/// First 1 MB reserved
///
/// Don't use before PMM initialization!
static DMA_MEMORY_BUDDY_ALLOCATOR: Mutex<BuddyAlloc> = Mutex::new(BuddyAlloc {
    buddy_ptr: core::ptr::null_mut::<buddy_alloc::buddy_alloc_sys::buddy>(),
});

/// Address of the first page of the DMA memory (first page of 2nd MB)
const DMA_MEMORY_FIRST_PAGE_ADDR: PhysAddr = PhysAddr::new(0x100000);
/// Address of the last page of the DMA memory (last page of 16th MB)
const DMA_MEMORY_LAST_PAGE_ADDR: PhysAddr = PhysAddr::new(0xFFF000);
/// DMA memory address space size (15 MB)
const DMA_MEMORY_ADDRESS_RANGE_SIZE: usize = DMA_MEMORY_LAST_PAGE_ADDR.as_u64() as usize + PAGE_SIZE - DMA_MEMORY_FIRST_PAGE_ADDR.as_u64() as usize;

/// Buddy Allocator for memory upper DMA zone (first 16 MB), main allocator
///
/// Don't use before PMM initialization!
static BUDDY_ALLOCATOR: Mutex<BuddyAlloc> = Mutex::new(BuddyAlloc {
    buddy_ptr: core::ptr::null_mut::<buddy_alloc::buddy_alloc_sys::buddy>(),
});

/// Inits Physical Memory Manager
pub fn init(boot_info: &bootloader_api::BootInfo) {
    // Init buddy allocators

    // Init DMA buddy allocator
    // Detect allocator range size: from first page, to last page (in available ranges)
    // Calculate metadata size
    // Find place for metadata at available physical memory chunk
    // Mark not available memory as allocated
    unsafe {
        let metadata_size = BuddyAlloc::sizeof_alignment(DMA_MEMORY_ADDRESS_RANGE_SIZE, PAGE_SIZE);
        serial_println!("Needed for DMA buddy alloc metadata: {metadata_size:?}");
    }

    // Init main buddy allocator
}
