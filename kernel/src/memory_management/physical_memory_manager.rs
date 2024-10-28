use bootloader_api::info::MemoryRegionKind;
use super::PAGE_SIZE;
use spin::Mutex;
use x86_64::PhysAddr;
use buddy_alloc::BuddyAlloc;
use lazy_static::lazy_static;
use tinyvec::ArrayVec;

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

/// Buddy Allocator for memory upper DMA zone (first 16 MB), main allocator
///
/// Don't use before PMM initialization!
static BUDDY_ALLOCATOR: Mutex<BuddyAlloc> = Mutex::new(BuddyAlloc {
    buddy_ptr: core::ptr::null_mut::<buddy_alloc::buddy_alloc_sys::buddy>(),
});

#[derive(Debug)]
/// Can be used by memory allocators
struct UsableRegion {
    /// First usable page
    ///
    /// Page-alligned
    first_page: PhysAddr,
    /// Last usable page
    ///
    /// Page-alligned
    last_page: PhysAddr,
}

impl Default for UsableRegion {
    fn default() -> Self {
        Self {
            first_page: PhysAddr::zero(),
            last_page: PhysAddr::zero(),
        }
    }
}

lazy_static! {
    /// Memory regions that's may be used by allocators, sorted
    static ref USABLE_REGIONS: Mutex<ArrayVec<[UsableRegion; 128]>> = {
        Mutex::new(ArrayVec::new())
    };
}

/// Inits Physical Memory Manager
pub fn init(boot_info: &bootloader_api::BootInfo) {
    // Collect usable regions data
    log::info!("Collecting usable regions data");
    let mut usable_regions_list = USABLE_REGIONS.lock();
    for usable_region in boot_info.memory_regions.iter().filter(|usable_region| usable_region.kind == MemoryRegionKind::Usable) {
        let mut start = usable_region.start;
        let end = usable_region.end;
        if start < DMA_MEMORY_FIRST_PAGE_ADDR.as_u64() {
            start = DMA_MEMORY_FIRST_PAGE_ADDR.as_u64();
            if end < start + PAGE_SIZE {
                // Region fully in first MB, skip
                log::debug!("Usable region {{start: 0x{:X}, end: 0x{:X}}}, dropped. Fully in first MB", start, end);
                continue;
            }
        }
        // Aligning addresses of too small a region can lead to problems, it is easier to discard it.
        if end - start < PAGE_SIZE * 4 {
            log::debug!("Usable region {{start: 0x{:X}, end 0x{:X}}}, dropped. Too small ({}).", start, end, end - start);
            continue;
        }
        // First usable page
        let first_page = PhysAddr::new(start).align_up(PAGE_SIZE);
        // Last usable page
        // end - PAGE_SIZE needed because end is exclusive
        let last_page = PhysAddr::new(end - PAGE_SIZE).align_down(PAGE_SIZE);
        assert!(last_page > first_page);
        assert!(last_page - first_page >= 4096);
        usable_regions_list.push(UsableRegion {
            first_page,
            last_page,
        })
    }
    // Sort
    usable_regions_list.as_mut_slice().sort_unstable_by(|a, b| {
        a.first_page.cmp(&b.first_page)
    });
    log::info!("Usable regions list:");
    for usable_region in usable_regions_list.iter() {
        log::info!("First page: 0x{:X}, Last page: 0x{:X}, Size: {}", usable_region.first_page.as_u64(), usable_region.last_page.as_u64(), usable_region.last_page + PAGE_SIZE - usable_region.first_page);
    }
    drop(usable_regions_list);

    // Init buddy allocators
    // Allocator initing:
    // 1. Detect allocator range size: from first usable page, to last usable page
    // 2. Calculate metadata size
    // 3. Find place for metadata at usable physical memory chunk
    // 4. Mark all memory as allocated
    // 5. Mark available memory as free

    // Init DMA buddy allocator

    // Init main buddy allocator
}
