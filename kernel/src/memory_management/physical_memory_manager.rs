use bootloader_api::info::MemoryRegionKind;
use super::PAGE_SIZE;
use spin::Mutex;
use x86_64::PhysAddr;
use buddy_alloc::BuddyAlloc;
use lazy_static::lazy_static;
use tinyvec::ArrayVec;

// DMA

/// Allocator for DMA addresses (1 MB - 16 MB range), secondary allocator
/// May be used if main allocator don't have memory
/// First 1 MB reserved
///
/// Don't use before PMM initialization!
static ALLOCATOR_DMA: Mutex<BuddyAlloc> = Mutex::new(BuddyAlloc {
    buddy_ptr: core::ptr::null_mut::<buddy_alloc::buddy_alloc_sys::buddy>(),
});

/// Reserved metadata for DMA allocator
///
/// 16 MB with 4 KB pages requires ~3 KB
static mut ALLOCATOR_DMA_METADATA: [u8; 1024 * 3] = [0; 1024 * 3];

/// Address of the first page of the DMA memory (first page of 2nd MB)
const DMA_FIRST_PAGE_ADDR: PhysAddr = PhysAddr::new(0x100000);

/// Address of the last page of the DMA memory (last page of 15th MB)
const DMA_LAST_PAGE_ADDR: PhysAddr = PhysAddr::new(0xFFF000);

/// DMA memory size
const DMA_SIZE: usize = (DMA_LAST_PAGE_ADDR.as_u64() + PAGE_SIZE - DMA_FIRST_PAGE_ADDR.as_u64()) as usize;

// DMA32

/// Allocator for DMA32 addresses (16 MB - 4 GB range), secondary allocator
/// May be used if main allocator don't have memory
///
/// Don't use before PMM initialization!
static ALLOCATOR_DMA32: Mutex<BuddyAlloc> = Mutex::new(BuddyAlloc {
    buddy_ptr: core::ptr::null_mut::<buddy_alloc::buddy_alloc_sys::buddy>(),
});

/// Reserved metadata for DMA32 allocator
///
/// 4 GB with 4 KB pages requires ~513 KB
static mut ALLOCATOR_DMA32_METADATA: [u8; 1024 * 513] = [0; 1024 * 513];

/// Address of the first page of the DMA32 memory (first page of 16th MB)
const DMA32_FIRST_PAGE_ADDR: PhysAddr = PhysAddr::new(0x1000000);

/// Address of the last page of the DMA32 memory (last page of 4th GB)
const DMA32_LAST_PAGE_ADDR: PhysAddr = PhysAddr::new(0xFFFFF000);

/// DMA memory size
const DMA32_SIZE: usize = (DMA32_LAST_PAGE_ADDR.as_u64() + PAGE_SIZE - DMA32_FIRST_PAGE_ADDR.as_u64()) as usize;

// Main

/// Address of the first page of the MAIN memory (first page of 5th GB)
const MAIN_FIRST_PAGE_ADDR: PhysAddr = PhysAddr::new(0x100000000);

/// Address of the last page of the MAIN memory (last page of 1st TB)
const MAIN_LAST_PAGE_ADDR: PhysAddr = PhysAddr::new(0xFFFFFFF000);

/// MAIN memory size
const MAIN_SIZE: usize = (MAIN_LAST_PAGE_ADDR.as_u64() + PAGE_SIZE - MAIN_FIRST_PAGE_ADDR.as_u64()) as usize;

/// Buddy Allocator for MAIN addresses (4 GB - 1 TB), main allocator
///
/// Don't use before PMM initialization!
static ALLOCATOR_MAIN: Mutex<BuddyAlloc> = Mutex::new(BuddyAlloc {
    buddy_ptr: core::ptr::null_mut::<buddy_alloc::buddy_alloc_sys::buddy>(),
});

#[derive(Debug, Copy, Clone)]
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

    /// Memory regions that's may be used by DMA allocator
    ///
    /// Fully in DMA memory, not in DMA32 or MAIN, sorted
    static ref DMA_USABLE_REGIONS: Mutex<ArrayVec<[UsableRegion; 128]>> = {
        Mutex::new(ArrayVec::new())
    };

    /// Memory regions that's may be used by DMA32 allocator
    ///
    /// Fully in DMA32, but not in DMA or MAIN memory, sorted
    static ref DMA32_USABLE_REGIONS: Mutex<ArrayVec<[UsableRegion; 128]>> = {
        Mutex::new(ArrayVec::new())
    };

    /// Memory regions that's may be used by main allocator
    ///
    /// MAIN memory, not in DMA or DMA32 memory, sorted
    static ref MAIN_USABLE_REGIONS: Mutex<ArrayVec<[UsableRegion; 128]>> = {
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
        if start < DMA_FIRST_PAGE_ADDR.as_u64() {
            start = DMA_FIRST_PAGE_ADDR.as_u64();
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

    // Collect usable DMA regions
    for usable_region in usable_regions_list.iter() {
        let new_usable_region = adjust_usable_region(usable_region, DMA_FIRST_PAGE_ADDR, DMA_LAST_PAGE_ADDR);
        if let Some(new_usable_region) = new_usable_region {
            debug_assert!(new_usable_region.first_page <= new_usable_region.last_page);
            debug_assert!(new_usable_region.first_page >= DMA_FIRST_PAGE_ADDR);
            debug_assert!(new_usable_region.last_page <= DMA_LAST_PAGE_ADDR);
            DMA_USABLE_REGIONS.lock().push(new_usable_region);
        }
    }

    // Collect usable DMA32 regions
    for usable_region in usable_regions_list.iter() {
        let new_usable_region = adjust_usable_region(usable_region, DMA32_FIRST_PAGE_ADDR, DMA32_LAST_PAGE_ADDR);
        if let Some(new_usable_region) = new_usable_region {
            debug_assert!(new_usable_region.first_page <= new_usable_region.last_page);
            debug_assert!(new_usable_region.first_page >= DMA32_FIRST_PAGE_ADDR);
            debug_assert!(new_usable_region.last_page <= DMA32_LAST_PAGE_ADDR);
            DMA32_USABLE_REGIONS.lock().push(new_usable_region);
        }
    }

    // Collect usable MAIN regions
    for usable_region in usable_regions_list.iter() {
        let new_usable_region = adjust_usable_region(usable_region, MAIN_FIRST_PAGE_ADDR, MAIN_LAST_PAGE_ADDR);
        if let Some(new_usable_region) = new_usable_region {
            debug_assert!(new_usable_region.first_page <= new_usable_region.last_page);
            debug_assert!(new_usable_region.first_page >= MAIN_FIRST_PAGE_ADDR);
            debug_assert!(new_usable_region.last_page <= MAIN_LAST_PAGE_ADDR);
            MAIN_USABLE_REGIONS.lock().push(new_usable_region);
        }
    }
    drop(usable_regions_list);
    // Debug checks
    #[cfg(debug_assertions)]
    {
        for v in USABLE_REGIONS.lock().iter() {
            assert!(v.first_page.is_aligned(PAGE_SIZE));
            assert!(v.last_page.is_aligned(PAGE_SIZE));
            assert!(v.first_page <= v.last_page);
        }
        for v in DMA_USABLE_REGIONS.lock().iter() {
            assert!(v.first_page.is_aligned(PAGE_SIZE));
            assert!(v.last_page.is_aligned(PAGE_SIZE));
            assert!(v.first_page <= v.last_page);
        }
        for v in DMA32_USABLE_REGIONS.lock().iter() {
            assert!(v.first_page.is_aligned(PAGE_SIZE));
            assert!(v.last_page.is_aligned(PAGE_SIZE));
            assert!(v.first_page <= v.last_page);
        }
        for v in MAIN_USABLE_REGIONS.lock().iter() {
            assert!(v.first_page.is_aligned(PAGE_SIZE));
            assert!(v.last_page.is_aligned(PAGE_SIZE));
            assert!(v.first_page <= v.last_page);
        }
    }

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

/// If usable_region can be entered in min and max, then the entered, reduced, region will be returned, otherwise None.
fn adjust_usable_region(
    usable_region: &UsableRegion,
    limit_first_page: PhysAddr,
    limit_last_page: PhysAddr,
) -> Option<UsableRegion> {
    if usable_region.last_page < limit_first_page || usable_region.first_page > limit_last_page {
        return None;
    }

    let mut new_usable_region = *usable_region;
    if usable_region.first_page < limit_first_page {
        new_usable_region.first_page = limit_first_page;
    }
    if usable_region.last_page > limit_last_page {
        new_usable_region.last_page = limit_last_page;
    }

    Some(new_usable_region)
}

