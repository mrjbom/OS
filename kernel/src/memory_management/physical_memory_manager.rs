use super::PAGE_SIZE;
use bootloader_api::info::MemoryRegionKind;
use buddy_alloc::BuddyAlloc;
use core::ptr::null_mut;
use lazy_static::lazy_static;
use spin::{Mutex, Once};
use tinyvec::ArrayVec;
use x86_64::PhysAddr;

/// Some zone in memory:
///
/// ISA DMA ZONE (1-16 MB)
///
/// DMA32 ZONE (16 MB - 4 GB)
///
/// HIGH DMA ZONE (4 GB - 1 TB)
// TODO: Implement Debug trait
struct MemoryZone {
    // Buddy allocator
    pub allocator: BuddyAlloc,
    // Statistics
}

#[derive(Debug, Copy, Clone)]
pub enum MemoryZoneEnum {
    IsaDma,
    Dma32,
    High,
}

/// Specifies from which zones memory can be allocated and the priority in which it should be allocated
///
/// Example:
/// [High, Dma32, IsaDma]:
/// Attempts to allocate memory first from HIGH, then from DMA32 and then from ISA DMA
///
/// or
///
/// [Dma32, High]:
/// Attempts to allocate memory first from Dma32, then from HIGH, but not trying to allocate memory from ISA DMA
type MemoryZonesAndPrioritySpecifier = [MemoryZoneEnum];

// ISA DMA

/// ISA DMA zone: 1 MB - 16 GB
///
/// First usable page: 0x100000
///
/// Last priority for allocations
///
/// Last usable page: 0xFFF000
static ISA_DMA_ZONE: Once<Mutex<MemoryZone>> = Once::new();

/// Reserved metadata for ISA DMA allocator
///
/// 16 MB with 4 KB pages requires ~3 KB
static mut ISA_DMA_ALLOCATOR_METADATA: [u8; 1024 * 3] = [0; 1024 * 3];

/// Address of the first page of the ISA DMA memory (first page of 2nd MB)
const ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR: PhysAddr = PhysAddr::new(0x100000);

/// Address of the last page of the DMA memory (last page of 15th MB)
const ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR: PhysAddr = PhysAddr::new(0xFFF000);

/// ISA DMA memory size
#[allow(unused)]
const ISA_DMA_ZONE_MAX_SIZE: usize = (ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR.as_u64() + PAGE_SIZE
    - ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR.as_u64()) as usize;

// DMA32

/// DMA32 zone: 16 MB - 4 GB
///
/// PCI
///
/// Second priority for allocations
///
/// First usable page: 0x1000000
///
/// Last usable page: 0xFFFF_F000
static DMA32_ZONE: Once<Mutex<MemoryZone>> = Once::new();

/// Reserved metadata for DMA32 allocator
///
/// 4 GB with 4 KB pages requires ~513 KB
static mut DMA32_ALLOCATOR_METADATA: [u8; 1024 * 513] = [0; 1024 * 513];

/// Address of the first page of the DMA32 memory (first page of 16th MB)
const DMA32_MIN_FIRST_PAGE_ADDR: PhysAddr = PhysAddr::new(0x1000000);

/// Address of the last page of the DMA32 memory (last page of 4th GB)
const DMA32_MAX_LAST_PAGE_ADDR: PhysAddr = PhysAddr::new(0xFFFF_F000);

/// DMA memory size
#[allow(unused)]
const DMA32_MAX_SIZE: usize =
    (DMA32_MAX_LAST_PAGE_ADDR.as_u64() + PAGE_SIZE - DMA32_MIN_FIRST_PAGE_ADDR.as_u64()) as usize;

// HIGH

/// HIGH zone: 4 GB - 1 TB
///
/// Max priority for allocations
///
/// First usable page: 0x1_0000_0000
///
/// Last usable page: 0xFF_FFFF_F000
static HIGH_ZONE: Once<Mutex<MemoryZone>> = Once::new();

/// Address of the first page of the HIGH memory (first page of 5th GB)
const HIGH_ZONE_MIN_FIRST_PAGE_ADDR: PhysAddr = PhysAddr::new(0x1_0000_0000);

/// Address of the last page of the HIGH memory (last page of 1st TB)
const HIGH_ZONE_MAX_LAST_PAGE_ADDR: PhysAddr = PhysAddr::new(0xFF_FFFF_F000);

/// HIGH memory size
#[allow(unused)]
const HIGH_ZONE_MAX_SIZE: usize = (HIGH_ZONE_MAX_LAST_PAGE_ADDR.as_u64() + PAGE_SIZE
    - HIGH_ZONE_MIN_FIRST_PAGE_ADDR.as_u64()) as usize;

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

    /// Memory regions that's may be used by ISA DMA allocator
    ///
    /// Fully in ISA DMA memory, not in DMA32 or MAIN, sorted
    static ref ISA_DMA_USABLE_REGIONS: Mutex<ArrayVec<[UsableRegion; 128]>> = {
        Mutex::new(ArrayVec::new())
    };

    /// Memory regions that's may be used by DMA32 allocator
    ///
    /// Fully in DMA32, but not in DMA or MAIN memory, sorted
    static ref DMA32_USABLE_REGIONS: Mutex<ArrayVec<[UsableRegion; 128]>> = {
        Mutex::new(ArrayVec::new())
    };

    /// Memory regions that's may be used by HIGH allocator
    ///
    /// HIGH memory, not in DMA or DMA32 memory, sorted
    static ref HIGH_USABLE_REGIONS: Mutex<ArrayVec<[UsableRegion; 128]>> = {
        Mutex::new(ArrayVec::new())
    };
}

/// Inits Physical Memory Manager and allocators
pub fn init(boot_info: &bootloader_api::BootInfo) {
    // Collect usable regions data
    log::info!("Collecting usable regions data");
    {
        let mut usable_regions_list = USABLE_REGIONS.lock();
        for usable_region in boot_info
            .memory_regions
            .iter()
            .filter(|usable_region| usable_region.kind == MemoryRegionKind::Usable)
        {
            let mut start = usable_region.start;
            let end = usable_region.end;
            if start < ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR.as_u64() {
                start = ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR.as_u64();
                if end <= start {
                    // Region fully in first MB, skip
                    log::debug!(
                        "Usable region {{start: 0x{:X}, end: 0x{:X}}}, dropped. Fully in first MB.",
                        usable_region.start,
                        usable_region.end
                    );
                    continue;
                }
            }
            // Aligning addresses of too small a region can lead to problems, it is easier to discard it.
            if end - start < PAGE_SIZE * 4 {
                log::debug!(
                    "Usable region {{start: 0x{:X}, end 0x{:X}}}, dropped. Too small ({}).",
                    usable_region.start,
                    usable_region.end,
                    usable_region.end - usable_region.start
                );
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
        usable_regions_list
            .as_mut_slice()
            .sort_unstable_by_key(|a| a.first_page);

        // Collect usable ISA DMA regions
        for usable_region in usable_regions_list.iter() {
            let new_usable_region = adjust_usable_region(
                usable_region,
                ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR,
                ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR,
            );
            if let Some(new_usable_region) = new_usable_region {
                debug_assert!(new_usable_region.first_page <= new_usable_region.last_page);
                debug_assert!(new_usable_region.first_page >= ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR);
                debug_assert!(new_usable_region.last_page <= ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR);
                ISA_DMA_USABLE_REGIONS.lock().push(new_usable_region);
            }
        }

        // Collect usable DMA32 regions
        for usable_region in usable_regions_list.iter() {
            let new_usable_region = adjust_usable_region(
                usable_region,
                DMA32_MIN_FIRST_PAGE_ADDR,
                DMA32_MAX_LAST_PAGE_ADDR,
            );
            if let Some(new_usable_region) = new_usable_region {
                debug_assert!(new_usable_region.first_page <= new_usable_region.last_page);
                debug_assert!(new_usable_region.first_page >= DMA32_MIN_FIRST_PAGE_ADDR);
                debug_assert!(new_usable_region.last_page <= DMA32_MAX_LAST_PAGE_ADDR);
                DMA32_USABLE_REGIONS.lock().push(new_usable_region);
            }
        }

        // Collect usable HIGH regions
        for usable_region in usable_regions_list.iter() {
            let new_usable_region = adjust_usable_region(
                usable_region,
                HIGH_ZONE_MIN_FIRST_PAGE_ADDR,
                HIGH_ZONE_MAX_LAST_PAGE_ADDR,
            );
            if let Some(new_usable_region) = new_usable_region {
                debug_assert!(new_usable_region.first_page <= new_usable_region.last_page);
                debug_assert!(new_usable_region.first_page >= HIGH_ZONE_MIN_FIRST_PAGE_ADDR);
                debug_assert!(new_usable_region.last_page <= HIGH_ZONE_MAX_LAST_PAGE_ADDR);
                HIGH_USABLE_REGIONS.lock().push(new_usable_region);
            }
        }
        drop(usable_regions_list);
        // Debug checks
        #[cfg(debug_assertions)]
        {
            for v in USABLE_REGIONS.lock().iter() {
                assert!(v.first_page >= ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR);
                assert!(v.first_page.is_aligned(PAGE_SIZE));
                assert!(v.last_page.is_aligned(PAGE_SIZE));
                assert!(v.first_page <= v.last_page);
            }

            for v in ISA_DMA_USABLE_REGIONS.lock().iter() {
                assert!(v.first_page >= ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR);
                assert!(v.last_page <= ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR);
                assert!(v.first_page.is_aligned(PAGE_SIZE));
                assert!(v.last_page.is_aligned(PAGE_SIZE));
                assert!(v.first_page <= v.last_page);
            }
            for v in DMA32_USABLE_REGIONS.lock().iter() {
                assert!(v.first_page >= DMA32_MIN_FIRST_PAGE_ADDR);
                assert!(v.last_page <= DMA32_MAX_LAST_PAGE_ADDR);
                assert!(v.first_page.is_aligned(PAGE_SIZE));
                assert!(v.last_page.is_aligned(PAGE_SIZE));
                assert!(v.first_page <= v.last_page);
            }
            for v in HIGH_USABLE_REGIONS.lock().iter() {
                assert!(v.first_page >= HIGH_ZONE_MIN_FIRST_PAGE_ADDR);
                assert!(v.last_page <= HIGH_ZONE_MAX_LAST_PAGE_ADDR);
                assert!(v.first_page.is_aligned(PAGE_SIZE));
                assert!(v.last_page.is_aligned(PAGE_SIZE));
                assert!(v.first_page <= v.last_page);
            }
        }
    }

    // Init allocators
    // Allocator initing:
    // 1. Detect allocator range size: from first usable page, to last usable page
    // 2. Calculate metadata size
    // 3. Find place for metadata at usable physical memory chunk
    // 4. Init allocator with alignment
    // 5. Mark all memory as allocated
    // 6. Mark available memory as free

    // Init DMA allocator
    #[allow(static_mut_refs)]
    unsafe {
        let isa_dma_usable_regions = ISA_DMA_USABLE_REGIONS.lock();
        if isa_dma_usable_regions.len() != 0 {
            // 1
            let first_page = isa_dma_usable_regions.first().unwrap().first_page;
            let last_page = isa_dma_usable_regions.last().unwrap().last_page;
            let range_size = (last_page + PAGE_SIZE - first_page) as usize;

            // 2
            let metadata_size = BuddyAlloc::sizeof_alignment(range_size, PAGE_SIZE as usize)
                .expect("Failed to calculate metadata size for ISA DMA allocator!");
            assert!(metadata_size <= ISA_DMA_ALLOCATOR_METADATA.len());

            // 4
            ISA_DMA_ZONE.call_once(|| {
                Mutex::new(MemoryZone {
                    allocator: BuddyAlloc::init_alignment(
                        ISA_DMA_ALLOCATOR_METADATA.as_mut_ptr(),
                        first_page.as_u64() as *mut u8,
                        range_size,
                        PAGE_SIZE as usize,
                    )
                    .expect("Failed to init ISA DMA buddy allocator!"),
                })
            });

            // 5
            ISA_DMA_ZONE
                .get()
                .unwrap()
                .lock()
                .allocator
                .reserve_range(first_page.as_u64() as *mut u8, range_size);

            // 6
            for usable_region in isa_dma_usable_regions.iter() {
                let first_page = usable_region.first_page;
                let range_size = usable_region.last_page + PAGE_SIZE - usable_region.first_page;
                ISA_DMA_ZONE
                    .get()
                    .unwrap()
                    .lock()
                    .allocator
                    .unsafe_release_range(first_page.as_u64() as *mut u8, range_size as usize);
            }
            log::info!("ISA DMA allocator inited");
        } else {
            log::info!("ISA DMA allocator not inited. No memory.")
        }
    }

    // Init DMA32 allocator
    #[allow(static_mut_refs)]
    unsafe {
        let dma32_usable_regions = DMA32_USABLE_REGIONS.lock();
        if dma32_usable_regions.len() != 0 {
            // 1
            let first_page = dma32_usable_regions.first().unwrap().first_page;
            let last_page = dma32_usable_regions.last().unwrap().last_page;
            let range_size = (last_page + PAGE_SIZE - first_page) as usize;

            // 2
            let metadata_size = BuddyAlloc::sizeof_alignment(range_size, PAGE_SIZE as usize)
                .expect("Failed to calculate metadata size for DMA32 allocator!");
            assert!(metadata_size <= DMA32_ALLOCATOR_METADATA.len());

            // 4
            DMA32_ZONE.call_once(|| {
                Mutex::new(MemoryZone {
                    allocator: BuddyAlloc::init_alignment(
                        DMA32_ALLOCATOR_METADATA.as_mut_ptr(),
                        first_page.as_u64() as *mut u8,
                        range_size,
                        PAGE_SIZE as usize,
                    )
                    .expect("Failed to init DMA32 buddy allocator!"),
                })
            });

            // 5
            DMA32_ZONE
                .get()
                .unwrap()
                .lock()
                .allocator
                .reserve_range(first_page.as_u64() as *mut u8, range_size);

            // 6
            for usable_region in dma32_usable_regions.iter() {
                let first_page = usable_region.first_page;
                let range_size = usable_region.last_page + PAGE_SIZE - usable_region.first_page;
                DMA32_ZONE
                    .get()
                    .unwrap()
                    .lock()
                    .allocator
                    .unsafe_release_range(first_page.as_u64() as *mut u8, range_size as usize);
            }
            log::info!("DMA32 allocator inited");
        } else {
            log::info!("DMA32 allocator not inited. No memory.")
        }
    }

    // Init HIGH allocator
    unsafe {
        let high_usable_regions = HIGH_USABLE_REGIONS.lock();
        if high_usable_regions.len() != 0 {
            // 1
            let first_page = high_usable_regions.first().unwrap().first_page;
            let last_page = high_usable_regions.last().unwrap().last_page;
            let range_size = (last_page + PAGE_SIZE - first_page) as usize;

            // 2
            let metadata_size = BuddyAlloc::sizeof_alignment(range_size, PAGE_SIZE as usize)
                .expect("Failed to calculate metadata size for HIGH allocator!");

            // 3
            // For 32 GB with 4KB pages ~ 5 MB
            // Try to allocate memory using DMA32 and DMA allocator
            let high_allocator_metadata = 'metadata: {
                // Try to allocate memory from DMA32
                if let Some(dma32_zone) = DMA32_ZONE.get() {
                    let ptr = dma32_zone.lock().allocator.malloc(metadata_size);
                    if !ptr.is_null() {
                        break 'metadata ptr;
                    }
                }
                // Allocation from DMA32 failed, try to allocate from DMA
                if let Some(isa_dma_zone) = ISA_DMA_ZONE.get() {
                    let ptr = isa_dma_zone.lock().allocator.malloc(metadata_size);
                    if !ptr.is_null() {
                        break 'metadata ptr;
                    }
                }
                // Failed to allocate memory
                core::ptr::null_mut()
            };
            // If HIGH allocator initialization has started, it means that memory is more than 4 GB, and therefore we should definitely find memory in DMA32
            assert!(!high_allocator_metadata.is_null(), "Failed to allocate memory for HIGH allocator's metadata! It's impossible, looks like bug!");
            // 4
            HIGH_ZONE.call_once(|| {
                Mutex::new(MemoryZone {
                    allocator: BuddyAlloc::init_alignment(
                        high_allocator_metadata,
                        first_page.as_u64() as *mut u8,
                        range_size,
                        PAGE_SIZE as usize,
                    )
                    .expect("Failed to init HIGH buddy allocator!"),
                })
            });

            // 5
            HIGH_ZONE
                .get()
                .unwrap()
                .lock()
                .allocator
                .reserve_range(first_page.as_u64() as *mut u8, range_size);

            // 6
            for usable_region in high_usable_regions.iter() {
                let first_page = usable_region.first_page;
                let range_size = usable_region.last_page + PAGE_SIZE - usable_region.first_page;
                HIGH_ZONE
                    .get()
                    .unwrap()
                    .lock()
                    .allocator
                    .unsafe_release_range(first_page.as_u64() as *mut u8, range_size as usize);
            }
            log::info!("HIGH allocator inited");
        } else {
            log::info!("HIGH allocator not inited. No memory.")
        }
    }

    if ISA_DMA_ZONE.get().is_none() && DMA32_ZONE.get().is_none() && HIGH_ZONE.get().is_none() {
        panic!("Physical memory allocator initialization failed! All buddy allocators not inited!");
    }
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

/// Allocs memory from zone using buddy allocators
///
/// requested_size must be power of two
///
/// MemoryZonesAndPrioritySpecifier specifies from which zones memory can be allocated and the priority in which it should be allocated
///
/// May be slow because may wait lock
pub fn alloc(
    memory_zones_and_priority_specifier: &MemoryZonesAndPrioritySpecifier,
    requested_size: usize,
) -> *mut u8 {
    assert_ne!(requested_size, 0, "Trying to alloc zero sized block");
    assert!(
        requested_size.is_power_of_two(),
        "requested_size is non power of two"
    );

    for requested_memory_zone_specifier in memory_zones_and_priority_specifier.iter() {
        // Lock zone
        let requested_memory_zone = match requested_memory_zone_specifier {
            MemoryZoneEnum::IsaDma => &ISA_DMA_ZONE,
            MemoryZoneEnum::Dma32 => &DMA32_ZONE,
            MemoryZoneEnum::High => &HIGH_ZONE,
        };
        // Zone exist?
        if let Some(requested_memory_zone) = requested_memory_zone.get() {
            // Try to alloc memory from zone
            let allocated_ptr = unsafe {
                requested_memory_zone
                    .lock()
                    .allocator
                    .malloc(requested_size)
            };
            if !allocated_ptr.is_null() {
                return allocated_ptr;
            }
        }
    }
    null_mut()
}

/// Frees memory to buddy allocator
///
/// May be slow because may wait lock
pub fn free(freed_ptr: *mut u8, memory_zone_enum: MemoryZoneEnum) {
    assert!(!freed_ptr.is_null(), "Trying to free null pointer");
    let memory_zone = match memory_zone_enum {
        MemoryZoneEnum::IsaDma => &ISA_DMA_ZONE,
        MemoryZoneEnum::Dma32 => &DMA32_ZONE,
        MemoryZoneEnum::High => &HIGH_ZONE,
    };
    unsafe {
        memory_zone
            .get()
            .expect("Trying to free memory from non-existing zone")
            .lock()
            .allocator
            .free(freed_ptr);
    }
}
