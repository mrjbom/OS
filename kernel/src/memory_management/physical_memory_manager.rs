use super::{virtual_memory_manager, PAGE_SIZE};
use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use buddy_alloc::BuddyAlloc;
use core::mem::MaybeUninit;
use core::ptr::null_mut;
use lazy_static::lazy_static;
use slab_allocator_lib::SlabInfo;
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
    /// (1-16 MB)
    IsaDma,
    /// (16 MB - 4 GB)
    Dma32,
    /// (4 GB - 1 TB)
    High,
}

/// Specifies from which zones memory can be allocated and the priority in which it should be allocated
///
/// Example:<br>
/// [High, Dma32, IsaDma]:
/// Attempts to allocate memory first from HIGH, then from DMA32 and then from ISA DMA<br>
/// or<br>
/// [Dma32, High]:
/// Attempts to allocate memory first from Dma32, then from HIGH, but not trying to allocate memory from ISA DMA<br>
type MemoryZonesAndPrioritySpecifier = [MemoryZoneEnum];

// ISA DMA

/// ISA DMA zone: 1 MB - 16 GB
///
/// First usable page: 0x100000
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
const ISA_DMA_ZONE_MAX_SIZE: usize = (ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR.as_u64() as usize
    + PAGE_SIZE
    - ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR.as_u64() as usize);

// DMA32

/// DMA32 zone: 16 MB - 4 GB
///
/// PCI
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
const DMA32_MAX_SIZE: usize = (DMA32_MAX_LAST_PAGE_ADDR.as_u64() as usize + PAGE_SIZE
    - DMA32_MIN_FIRST_PAGE_ADDR.as_u64() as usize);

// HIGH

/// HIGH zone: 4 GB - 1 TB
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
const HIGH_ZONE_MAX_SIZE: usize = (HIGH_ZONE_MAX_LAST_PAGE_ADDR.as_u64() as usize + PAGE_SIZE
    - HIGH_ZONE_MIN_FIRST_PAGE_ADDR.as_u64() as usize);

#[derive(Debug, Copy, Clone, PartialEq)]
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

impl UsableRegion {
    /// Gets size of usable region
    pub fn size(&self) -> usize {
        self.last_page.as_u64() as usize + PAGE_SIZE - self.first_page.as_u64() as usize
    }
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
    collect_usable_regions(&boot_info.memory_regions);
    init_slab_info_ptrs_array();
    init_allocators();

    // Check lists
    assert!(USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.first_page));
    assert!(USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.last_page));
    assert!(USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.size() >= PAGE_SIZE));

    assert!(ISA_DMA_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.first_page));
    assert!(ISA_DMA_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.last_page));
    assert!(ISA_DMA_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.size() >= PAGE_SIZE));

    assert!(DMA32_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.first_page));
    assert!(DMA32_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.last_page));
    assert!(DMA32_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.size() >= PAGE_SIZE));

    assert!(HIGH_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.first_page));
    assert!(HIGH_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.last_page));
    assert!(HIGH_USABLE_REGIONS
        .lock()
        .iter()
        .is_sorted_by_key(|v| v.size() >= PAGE_SIZE));

    assert_eq!(
        USABLE_REGIONS.lock().len(),
        ISA_DMA_USABLE_REGIONS.lock().len()
            + DMA32_USABLE_REGIONS.lock().len()
            + HIGH_USABLE_REGIONS.lock().len()
    );

    // Checks if the region is in more than in one zone at the same time.
    for some_region in USABLE_REGIONS.lock().iter() {
        let mut was_found_n_times = 0;
        if ISA_DMA_USABLE_REGIONS
            .lock()
            .iter()
            .any(|dma_region| some_region == dma_region)
        {
            was_found_n_times += 1;
        }
        if DMA32_USABLE_REGIONS
            .lock()
            .iter()
            .any(|dma32_region| some_region == dma32_region)
        {
            was_found_n_times += 1;
        }
        if HIGH_USABLE_REGIONS
            .lock()
            .iter()
            .any(|high_region| some_region == high_region)
        {
            was_found_n_times += 1;
        }
        assert_eq!(was_found_n_times, 1);
    }

    // Check free memory in allocator and regions
    if let Some(zone) = ISA_DMA_ZONE.get() {
        let free_memory_size: usize = ISA_DMA_USABLE_REGIONS.lock().iter().map(|v| v.size()).sum();
        unsafe {
            assert_eq!(zone.lock().allocator.arena_free_size(), free_memory_size);
        }
    }

    if let Some(zone) = DMA32_ZONE.get() {
        let free_memory_size: usize = DMA32_USABLE_REGIONS.lock().iter().map(|v| v.size()).sum();
        unsafe {
            assert_eq!(zone.lock().allocator.arena_free_size(), free_memory_size);
        }
    }

    if let Some(zone) = HIGH_ZONE.get() {
        let free_memory_size: usize = HIGH_USABLE_REGIONS.lock().iter().map(|v| v.size()).sum();
        unsafe {
            assert_eq!(zone.lock().allocator.arena_free_size(), free_memory_size);
        }
    }
}

/// Parses memory map and collects data about usable regions
fn collect_usable_regions(memory_regions: &[MemoryRegion]) {
    // Collect all usable regions
    let mut usable_regions_lock = USABLE_REGIONS.lock();
    for usable_region in memory_regions
        .iter()
        .filter(|usable_region| usable_region.kind == MemoryRegionKind::Usable)
    {
        if usable_region.start < ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR.as_u64()
            || usable_region.end > HIGH_ZONE_MAX_LAST_PAGE_ADDR.as_u64()
        {
            continue;
        }

        let mut first_page = PhysAddr::new(usable_region.start);
        first_page = first_page.align_up(PAGE_SIZE as u64);
        let mut last_page = PhysAddr::new(usable_region.end);
        last_page -= PAGE_SIZE as u64;
        last_page = last_page.align_down(PAGE_SIZE as u64);

        if last_page <= first_page {
            continue;
        }
        if last_page - first_page < PAGE_SIZE as u64 {
            continue;
        }
        assert!(first_page.is_aligned(PAGE_SIZE as u64));
        assert!(last_page.is_aligned(PAGE_SIZE as u64));

        usable_regions_lock.push(UsableRegion {
            first_page,
            last_page,
        })
    }
    // Sort
    usable_regions_lock
        .as_mut_slice()
        .sort_unstable_by_key(|a| a.first_page);

    // Collect usable ISA DMA regions
    for usable_region in usable_regions_lock.iter() {
        let new_usable_region = adjust_usable_region(
            usable_region,
            ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR,
            ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR,
        );
        if let Some(new_usable_region) = new_usable_region {
            assert!(new_usable_region.first_page <= new_usable_region.last_page);
            assert!(new_usable_region.first_page >= ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR);
            assert!(new_usable_region.last_page <= ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR);
            ISA_DMA_USABLE_REGIONS.lock().push(new_usable_region);
        }
    }

    // Collect usable DMA32 regions
    for usable_region in usable_regions_lock.iter() {
        let new_usable_region = adjust_usable_region(
            usable_region,
            DMA32_MIN_FIRST_PAGE_ADDR,
            DMA32_MAX_LAST_PAGE_ADDR,
        );
        if let Some(new_usable_region) = new_usable_region {
            assert!(new_usable_region.first_page <= new_usable_region.last_page);
            assert!(new_usable_region.first_page >= DMA32_MIN_FIRST_PAGE_ADDR);
            assert!(new_usable_region.last_page <= DMA32_MAX_LAST_PAGE_ADDR);
            DMA32_USABLE_REGIONS.lock().push(new_usable_region);
        }
    }

    // Collect usable HIGH regions
    for usable_region in usable_regions_lock.iter() {
        let new_usable_region = adjust_usable_region(
            usable_region,
            HIGH_ZONE_MIN_FIRST_PAGE_ADDR,
            HIGH_ZONE_MAX_LAST_PAGE_ADDR,
        );
        if let Some(new_usable_region) = new_usable_region {
            assert!(new_usable_region.first_page <= new_usable_region.last_page);
            assert!(new_usable_region.first_page >= HIGH_ZONE_MIN_FIRST_PAGE_ADDR);
            assert!(new_usable_region.last_page <= HIGH_ZONE_MAX_LAST_PAGE_ADDR);
            HIGH_USABLE_REGIONS.lock().push(new_usable_region);
        }
    }
    // Debug checks
    #[cfg(debug_assertions)]
    {
        drop(usable_regions_lock);
        for v in USABLE_REGIONS.lock().iter() {
            assert!(v.first_page >= ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR);
            assert!(v.first_page.is_aligned(PAGE_SIZE as u64));
            assert!(v.last_page.is_aligned(PAGE_SIZE as u64));
            assert!(v.first_page <= v.last_page);
        }

        for v in ISA_DMA_USABLE_REGIONS.lock().iter() {
            assert!(v.first_page >= ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR);
            assert!(v.last_page <= ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR);
            assert!(v.first_page.is_aligned(PAGE_SIZE as u64));
            assert!(v.last_page.is_aligned(PAGE_SIZE as u64));
            assert!(v.first_page <= v.last_page);
        }
        for v in DMA32_USABLE_REGIONS.lock().iter() {
            assert!(v.first_page >= DMA32_MIN_FIRST_PAGE_ADDR);
            assert!(v.last_page <= DMA32_MAX_LAST_PAGE_ADDR);
            assert!(v.first_page.is_aligned(PAGE_SIZE as u64));
            assert!(v.last_page.is_aligned(PAGE_SIZE as u64));
            assert!(v.first_page <= v.last_page);
        }
        for v in HIGH_USABLE_REGIONS.lock().iter() {
            assert!(v.first_page >= HIGH_ZONE_MIN_FIRST_PAGE_ADDR);
            assert!(v.last_page <= HIGH_ZONE_MAX_LAST_PAGE_ADDR);
            assert!(v.first_page.is_aligned(PAGE_SIZE as u64));
            assert!(v.last_page.is_aligned(PAGE_SIZE as u64));
            assert!(v.first_page <= v.last_page);
        }
    }
}

/// If usable_region can be entered in min and max, then the entered, reduced, region will be returned, otherwise None.
fn adjust_usable_region(
    usable_region: &UsableRegion,
    limit_first_page: PhysAddr,
    limit_last_page: PhysAddr,
) -> Option<UsableRegion> {
    assert!(usable_region.size() >= PAGE_SIZE);
    assert!(limit_first_page < limit_last_page);

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

/// Inits array of SlabInfo pointers
fn init_slab_info_ptrs_array() {
    // Calculate required memory size for store SlabInfo's
    // SlabInfo per page from first usable to last usable
    let first_usable_page_addr = USABLE_REGIONS.lock().first().unwrap().first_page.as_u64();
    let last_usable_page_addr = USABLE_REGIONS.lock().last().unwrap().last_page.as_u64();
    let number_of_slab_infos =
        (last_usable_page_addr + PAGE_SIZE as u64 - first_usable_page_addr) as usize / PAGE_SIZE;

    let mut required_memory_size = number_of_slab_infos * size_of::<*mut SlabInfo>();
    required_memory_size = x86_64::align_up(required_memory_size as u64, PAGE_SIZE as u64) as usize;
    assert_eq!(required_memory_size % PAGE_SIZE, 0);

    // Reserve required memory in usable region
    // Physical address of the array
    let mut required_memory_phys_addr: PhysAddr = PhysAddr::zero();
    assert!(USABLE_REGIONS.lock().is_sorted_by_key(|v| { v.first_page }));
    for usable_region in USABLE_REGIONS.lock().iter_mut().rev() {
        if usable_region.size() >= required_memory_size + PAGE_SIZE {
            // Use this region
            required_memory_phys_addr = usable_region.first_page;
            usable_region.first_page += required_memory_size as u64;
            assert!(usable_region.first_page.is_aligned(PAGE_SIZE as u64));
            assert!(usable_region.size() >= PAGE_SIZE);

            // Don't forget to change data in other list
            for v in ISA_DMA_USABLE_REGIONS.lock().iter_mut() {
                if v.first_page == usable_region.first_page - required_memory_size as u64 {
                    v.first_page = usable_region.first_page;
                    assert!(v.size() >= PAGE_SIZE);
                    break;
                }
            }
            for v in DMA32_USABLE_REGIONS.lock().iter_mut() {
                if v.first_page == usable_region.first_page - required_memory_size as u64 {
                    v.first_page = usable_region.first_page;
                    assert!(v.size() >= PAGE_SIZE);
                    break;
                }
            }
            for v in HIGH_USABLE_REGIONS.lock().iter_mut() {
                if v.first_page == usable_region.first_page - required_memory_size as u64 {
                    v.first_page = usable_region.first_page;
                    assert!(v.size() >= PAGE_SIZE);
                    break;
                }
            }

            break;
        }
    }
    assert!(
        !required_memory_phys_addr.is_null(),
        "Failed to find memory for SlabInfo pointers array"
    );
    assert!(required_memory_phys_addr.is_aligned(align_of::<SlabInfo>() as u64));
    assert!(
        USABLE_REGIONS.lock().is_sorted_by_key(|v| { v.first_page }),
        "Usable regions sort broken, looks like bug (probably the memory map is not quite right)"
    );

    // Memory reserved, make slice
    // Convert to virtual address
    let required_memory_virt_addr =
        virtual_memory_manager::virt_addr_in_cpmm_from_phys_addr(required_memory_phys_addr);
    // We don't init memory, because it's may be slow operation, there is no UB, because MaybeUninit used.
    // For example, a machine with 32 gigabytes of memory will need to initialize 64 megabytes.
    let slice: &'static mut [MaybeUninit<*mut SlabInfo>] = unsafe {
        core::slice::from_raw_parts_mut(
            required_memory_virt_addr.as_mut_ptr(),
            number_of_slab_infos,
        )
    };
    assert_eq!(size_of_val(slice.first().unwrap()), size_of::<*mut u8>());

    #[allow(static_mut_refs)]
    unsafe {
        super::slab_allocator::SLAB_INFO_PTRS.call_once(|| slice);
    }
}

/// Inits zone allocators
fn init_allocators() {
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
        let isa_dma_usable_regions_lock = ISA_DMA_USABLE_REGIONS.lock();
        if isa_dma_usable_regions_lock.len() != 0 {
            // 1
            let first_page = isa_dma_usable_regions_lock.first().unwrap().first_page;
            let last_page = isa_dma_usable_regions_lock.last().unwrap().last_page;
            let range_size = (last_page + PAGE_SIZE as u64 - first_page) as usize;

            // 2
            let metadata_size = BuddyAlloc::sizeof_alignment(range_size, PAGE_SIZE)
                .expect("Failed to calculate metadata size for ISA DMA allocator!");
            assert!(metadata_size <= ISA_DMA_ALLOCATOR_METADATA.len());

            // 4
            ISA_DMA_ZONE.call_once(|| {
                Mutex::new(MemoryZone {
                    allocator: BuddyAlloc::init_alignment(
                        ISA_DMA_ALLOCATOR_METADATA.as_mut_ptr(),
                        first_page.as_u64() as *mut u8,
                        range_size,
                        PAGE_SIZE,
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
            for usable_region in isa_dma_usable_regions_lock.iter() {
                ISA_DMA_ZONE
                    .get()
                    .unwrap()
                    .lock()
                    .allocator
                    .unsafe_release_range(
                        usable_region.first_page.as_u64() as *mut u8,
                        usable_region.size(),
                    );
            }
            log::info!("ISA DMA allocator inited");
        } else {
            log::info!("ISA DMA allocator not inited. No memory.")
        }
    }

    // Init DMA32 allocator
    #[allow(static_mut_refs)]
    unsafe {
        let dma32_usable_regions_lock = DMA32_USABLE_REGIONS.lock();
        if dma32_usable_regions_lock.len() != 0 {
            // 1
            let first_page = dma32_usable_regions_lock.first().unwrap().first_page;
            let last_page = dma32_usable_regions_lock.last().unwrap().last_page;
            let range_size = (last_page + PAGE_SIZE as u64 - first_page) as usize;

            // 2
            let metadata_size = BuddyAlloc::sizeof_alignment(range_size, PAGE_SIZE)
                .expect("Failed to calculate metadata size for DMA32 allocator!");
            assert!(metadata_size <= DMA32_ALLOCATOR_METADATA.len());

            // 4
            DMA32_ZONE.call_once(|| {
                Mutex::new(MemoryZone {
                    allocator: BuddyAlloc::init_alignment(
                        DMA32_ALLOCATOR_METADATA.as_mut_ptr(),
                        first_page.as_u64() as *mut u8,
                        range_size,
                        PAGE_SIZE,
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
            for usable_region in dma32_usable_regions_lock.iter() {
                DMA32_ZONE
                    .get()
                    .unwrap()
                    .lock()
                    .allocator
                    .unsafe_release_range(
                        usable_region.first_page.as_u64() as *mut u8,
                        usable_region.size(),
                    );
            }
            log::info!("DMA32 allocator inited");
        } else {
            log::info!("DMA32 allocator not inited. No memory.")
        }
    }

    // Init HIGH allocator
    unsafe {
        let high_usable_regions_lock = HIGH_USABLE_REGIONS.lock();
        if high_usable_regions_lock.len() != 0 {
            // 1
            let first_page = high_usable_regions_lock.first().unwrap().first_page;
            let last_page = high_usable_regions_lock.last().unwrap().last_page;
            let range_size = (last_page + PAGE_SIZE as u64 - first_page) as usize;

            // 2
            let mut metadata_size = BuddyAlloc::sizeof_alignment(range_size, PAGE_SIZE)
                .expect("Failed to calculate metadata size for HIGH allocator!");
            metadata_size = x86_64::align_up(metadata_size as u64, PAGE_SIZE as u64) as usize;
            assert_eq!(metadata_size % PAGE_SIZE, 0);

            // 3
            // For 32 GB with 4KB pages ~ 5 MB
            // Allocate memory from DMA32
            let mut high_allocator_metadata: *mut u8 = null_mut();
            for usable_region in DMA32_USABLE_REGIONS.lock().iter_mut() {
                if usable_region.size() >= metadata_size + PAGE_SIZE {
                    high_allocator_metadata = usable_region.first_page.as_u64() as *mut u8;
                    usable_region.first_page += metadata_size as u64;
                    assert!(usable_region.first_page.is_aligned(PAGE_SIZE as u64));
                    assert!(usable_region.size() >= PAGE_SIZE);

                    // Don't forget to change data in other list
                    for v in USABLE_REGIONS.lock().iter_mut() {
                        if v.first_page == usable_region.first_page - metadata_size as u64 {
                            v.first_page = usable_region.first_page;
                            assert!(v.size() >= PAGE_SIZE);
                            break;
                        }
                    }

                    // Mark allocated memory in DMA32 allocator
                    DMA32_ZONE
                        .get()
                        .unwrap()
                        .lock()
                        .allocator
                        .reserve_range(high_allocator_metadata, metadata_size);
                    break;
                }
            }

            // If HIGH allocator initialization has started, it means that memory is more than 4 GB, and therefore we should definitely find memory in DMA32
            assert!(!high_allocator_metadata.is_null(), "Failed to allocate memory for HIGH allocator's metadata! It's impossible, looks like bug!");

            // Convert physical address to virtual
            let high_allocator_metadata = high_allocator_metadata
                .byte_add(virtual_memory_manager::PHYSICAL_MEMORY_MAPPING_OFFSET as usize);

            // 4
            HIGH_ZONE.call_once(|| {
                Mutex::new(MemoryZone {
                    allocator: BuddyAlloc::init_alignment(
                        high_allocator_metadata,
                        first_page.as_u64() as *mut u8,
                        range_size,
                        PAGE_SIZE,
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
            for usable_region in high_usable_regions_lock.iter() {
                HIGH_ZONE
                    .get()
                    .unwrap()
                    .lock()
                    .allocator
                    .unsafe_release_range(
                        usable_region.first_page.as_u64() as *mut u8,
                        usable_region.size(),
                    );
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

/// Allocs memory from zone using buddy allocators
///
/// request_size must be one or more pages
///
/// MemoryZonesAndPrioritySpecifier specifies from which zones memory can be allocated and the priority in which it should be allocated
///
/// May be slow because may wait lock
///
/// # Safety
/// May return null address<br>
/// Allocated memory is uninitialized
pub unsafe fn alloc(
    memory_zones_and_priority_specifier: &MemoryZonesAndPrioritySpecifier,
    requested_size: usize,
) -> PhysAddr {
    debug_assert!(
        requested_size >= PAGE_SIZE && requested_size.is_power_of_two(),
        "Requested size must be one or more pages"
    );

    for requested_memory_zone_specifier in memory_zones_and_priority_specifier.iter() {
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
                debug_assert_eq!(
                    allocated_ptr as usize % PAGE_SIZE,
                    0,
                    "Buddy allocator allocates non aligned address"
                );
                return PhysAddr::new(allocated_ptr as u64);
            }
        }
    }
    PhysAddr::zero()
}

/// Frees memory to buddy allocator
///
/// May be slow because may wait lock
///
/// # Safety
/// Freed memory must be previously allocated memory
pub unsafe fn free(freed_addr: PhysAddr) {
    debug_assert!(!freed_addr.is_null(), "Trying to free null address");
    debug_assert!(
        freed_addr.is_aligned(PAGE_SIZE as u64),
        "Trying to free non aligned address"
    );

    let memory_zone = get_zone_allocator_by_addr(freed_addr);

    unsafe {
        memory_zone
            .get()
            .expect("Trying to free memory from non-existing zone")
            .lock()
            .allocator
            .free(freed_addr.as_u64() as *mut u8);
    }
}

/// Reallocs memory, like C realloc
pub unsafe fn realloc(phys_addr: PhysAddr, requested_size: usize, ignore_data: bool) -> *mut u8 {
    if !ignore_data {
        unimplemented!("Since the buddy allocator works with physical memory, it will not be able to move data");
    }
    let memory_zone = get_zone_allocator_by_addr(phys_addr);

    memory_zone
        .get()
        .expect("Trying to free memory from non-existing zone")
        .lock()
        .allocator
        .realloc(phys_addr.as_u64() as *mut u8, requested_size, ignore_data)
}

fn get_zone_allocator_by_addr(phys_addr: PhysAddr) -> &'static Once<Mutex<MemoryZone>> {
    if phys_addr >= ISA_DMA_ZONE_MIN_FIRST_PAGE_ADDR && phys_addr <= ISA_DMA_ZONE_MAX_LAST_PAGE_ADDR
    {
        &ISA_DMA_ZONE
    } else if phys_addr >= DMA32_MIN_FIRST_PAGE_ADDR && phys_addr <= DMA32_MAX_LAST_PAGE_ADDR {
        &DMA32_ZONE
    } else if phys_addr >= HIGH_ZONE_MIN_FIRST_PAGE_ADDR
        && phys_addr <= HIGH_ZONE_MAX_LAST_PAGE_ADDR
    {
        &HIGH_ZONE
    } else {
        unreachable!("Trying to free invalid address");
    }
}
