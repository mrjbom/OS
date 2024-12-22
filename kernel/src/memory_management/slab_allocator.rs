use crate::memory_management::physical_memory_manager::MemoryZoneEnum;
use crate::memory_management::PAGE_SIZE;
use core::mem::MaybeUninit;
use core::ptr::null_mut;
use slab_allocator_lib::{Cache, MemoryBackend, ObjectSizeType, SlabInfo};
use spin::{Mutex, Once};
use x86_64::VirtAddr;

/// Array of saved SlabInfo's pointers for each page. Used by Slab Allocator's
///
/// Mutex is not required because a properly working SlabAllocator and his MemoryBackend will not touch data that is not its own
// But the hash table approach has the disadvantage that
// it requires expanding the hash table (doubling its size), and it doesn't fit well with SlabAllocator (you'll have to create a many of caches of double size)
// This array approach wastes memory. Thus, we have to store 262144 pointers(2097152 bytes, 2MB) for 1 GB of memory
//
// MaybeUninit is used because initializing the entire array memory before creating a slice is a heavy operation
pub static mut SLAB_INFO_PTRS: Once<&'static mut [MaybeUninit<*mut SlabInfo>]> = Once::new();

/// Cache with SlabInfo's
static SLAB_INFO_CACHE: Once<Mutex<Cache<SlabInfo, SlabInfoCacheMemoryBackend>>> = Once::new();

// Generic caches for generic allocator

/// Defines static generic cache
///
/// `generic_cache!(GENERIC_CACHE32_4, 32, 4, GenericCacheType32_4, DefaultMemoryBackend);`
macro_rules! define_static_generic_cache {
    ($cache_name:ident, $size:expr, $align:expr, $cache_type:ident, $memory_backend_type:ident) => {
        #[repr(C, align($align))]
        struct $cache_type {
            _data: [u8; $size],
        }
        static $cache_name: Once<Mutex<Cache<$cache_type, $memory_backend_type>>> = Once::new();
    };
}

define_static_generic_cache!(
    GENERIC_CACHE16,
    16,
    16,
    GenericCacheType16_16,
    DefaultMemoryBackend
);
define_static_generic_cache!(
    GENERIC_CACHE32,
    32,
    32,
    GenericCacheType32_32,
    DefaultMemoryBackend
);
define_static_generic_cache!(
    GENERIC_CACHE64,
    64,
    64,
    GenericCacheType64_64,
    DefaultMemoryBackend
);
define_static_generic_cache!(
    GENERIC_CACHE128,
    128,
    128,
    GenericCacheType128_128,
    DefaultMemoryBackend
);
define_static_generic_cache!(
    GENERIC_CACHE256,
    256,
    256,
    GenericCacheType256_256,
    DefaultMemoryBackend
);
define_static_generic_cache!(
    GENERIC_CACHE512,
    512,
    512,
    GenericCacheType512_512,
    DefaultMemoryBackend
);
define_static_generic_cache!(
    GENERIC_CACHE1024,
    1024,
    1024,
    GenericCacheType1024_1024,
    DefaultMemoryBackend
);
define_static_generic_cache!(
    GENERIC_CACHE2048,
    2048,
    2048,
    GenericCacheType2048_2048,
    DefaultMemoryBackend
);

/// Inits generic cache defined by define_static_generic_cache!() macro
///
/// init_static_generic_cache!(GENERIC_CACHE16, 4096, ObjectSizeType::Small, DefaultMemoryBackend);
macro_rules! init_static_generic_cache {
    ($cache_name:ident, $slab_size:expr, $object_size_type:path, $memory_backend_type:ident) => {
        $cache_name.call_once(|| {
            Mutex::new(
                Cache::new(
                    $slab_size,
                    PAGE_SIZE,
                    $object_size_type,
                    $memory_backend_type,
                )
                .unwrap_or_else(|error| panic!("Failed to create generic cache: {error}")),
            )
        });
    };
}

/// An allocator consisting of a set of caches of different sizes can be used as a general purpose allocator.
///
/// However, it may lead to excessive memory waste, so it should not be used for everything.
///
/// Separate caches should be used for kernel objects that are allocated a lot and frequently.
struct GenericAllocator;

/// Inits slab caches
pub fn init() {
    // Init SlabInfo cache
    SLAB_INFO_CACHE.call_once(|| {
        Mutex::new(
            Cache::new(
                4096,
                PAGE_SIZE,
                ObjectSizeType::Small,
                SlabInfoCacheMemoryBackend,
            )
            .unwrap_or_else(|error| panic!("Failed to create SlabInfo cache: {error}")),
        )
    });

    // Init generic caches
    // 16
    log::debug!("16");
    init_static_generic_cache!(
        GENERIC_CACHE16,
        4096,
        ObjectSizeType::Small,
        DefaultMemoryBackend
    );
    unsafe {
        let allocated_ptr = GENERIC_CACHE16.get().unwrap().lock().alloc();
        assert!(!allocated_ptr.is_null());
        assert!(allocated_ptr.is_aligned());
        allocated_ptr.write_bytes(0, 1);
        GENERIC_CACHE16.get().unwrap().lock().free(allocated_ptr);
    }

    // 32
    log::debug!("32");
    init_static_generic_cache!(
        GENERIC_CACHE32,
        4096,
        ObjectSizeType::Small,
        DefaultMemoryBackend
    );
    unsafe {
        let allocated_ptr = GENERIC_CACHE32.get().unwrap().lock().alloc();
        assert!(!allocated_ptr.is_null());
        assert!(allocated_ptr.is_aligned());
        allocated_ptr.write_bytes(0, 1);
        GENERIC_CACHE32.get().unwrap().lock().free(allocated_ptr);
    }

    // 64
    log::debug!("64");
    init_static_generic_cache!(
        GENERIC_CACHE64,
        4096,
        ObjectSizeType::Small,
        DefaultMemoryBackend
    );
    unsafe {
        let allocated_ptr = GENERIC_CACHE64.get().unwrap().lock().alloc();
        assert!(!allocated_ptr.is_null());
        assert!(allocated_ptr.is_aligned());
        allocated_ptr.write_bytes(0, 1);
        GENERIC_CACHE64.get().unwrap().lock().free(allocated_ptr);
    }

    // 128
    log::debug!("128");
    init_static_generic_cache!(
        GENERIC_CACHE128,
        4096,
        ObjectSizeType::Small,
        DefaultMemoryBackend
    );
    unsafe {
        let allocated_ptr = GENERIC_CACHE128.get().unwrap().lock().alloc();
        assert!(!allocated_ptr.is_null());
        assert!(allocated_ptr.is_aligned());
        allocated_ptr.write_bytes(0, 1);
        GENERIC_CACHE128.get().unwrap().lock().free(allocated_ptr);
    }

    // 256
    log::debug!("256");
    init_static_generic_cache!(
        GENERIC_CACHE256,
        4096,
        ObjectSizeType::Small,
        DefaultMemoryBackend
    );
    unsafe {
        let allocated_ptr = GENERIC_CACHE256.get().unwrap().lock().alloc();
        assert!(!allocated_ptr.is_null());
        assert!(allocated_ptr.is_aligned());
        allocated_ptr.write_bytes(0, 1);
        GENERIC_CACHE256.get().unwrap().lock().free(allocated_ptr);
    }

    // 512
    log::debug!("512");
    init_static_generic_cache!(
        GENERIC_CACHE512,
        4096,
        ObjectSizeType::Large,
        DefaultMemoryBackend
    );
    unsafe {
        let allocated_ptr = GENERIC_CACHE512.get().unwrap().lock().alloc();
        assert!(!allocated_ptr.is_null());
        assert!(allocated_ptr.is_aligned());
        allocated_ptr.write_bytes(0, 1);
        GENERIC_CACHE512.get().unwrap().lock().free(allocated_ptr);
    }

    // 1024
    log::debug!("1024");
    init_static_generic_cache!(
        GENERIC_CACHE1024,
        4096,
        ObjectSizeType::Large,
        DefaultMemoryBackend
    );
    unsafe {
        let allocated_ptr = GENERIC_CACHE1024.get().unwrap().lock().alloc();
        assert!(!allocated_ptr.is_null());
        assert!(allocated_ptr.is_aligned());
        allocated_ptr.write_bytes(0, 1);
        GENERIC_CACHE1024.get().unwrap().lock().free(allocated_ptr);
    }

    // 2048
    log::debug!("2048");
    init_static_generic_cache!(
        GENERIC_CACHE2048,
        4096,
        ObjectSizeType::Large,
        DefaultMemoryBackend
    );
    unsafe {
        let allocated_ptr = GENERIC_CACHE2048.get().unwrap().lock().alloc();
        assert!(!allocated_ptr.is_null());
        assert!(allocated_ptr.is_aligned());
        allocated_ptr.write_bytes(0, 1);
        GENERIC_CACHE2048.get().unwrap().lock().free(allocated_ptr);
    }
}

struct DefaultMemoryBackend;

impl MemoryBackend for DefaultMemoryBackend {
    unsafe fn alloc_slab(&mut self, slab_size: usize, page_size: usize) -> *mut u8 {
        debug_assert!(
            slab_size != 0 && slab_size.is_power_of_two() && slab_size % page_size == 0,
            "Slab allocator tries to allocate invalid slab size"
        );
        // Alloc physical frame with slab size
        let phys_addr = super::physical_memory_manager::alloc(
            &[
                MemoryZoneEnum::High,
                MemoryZoneEnum::IsaDma,
                MemoryZoneEnum::Dma32,
            ],
            slab_size,
        );
        log::debug!("allocated_slab_phys_addr: {phys_addr:p}");
        log::debug!(
            "allocated_slab_virt_addr: {:p}",
            super::virtual_memory_manager::phys_addr_to_cpmm_virt_addr(phys_addr)
        );
        if phys_addr.is_null() {
            return null_mut();
        }
        super::virtual_memory_manager::phys_addr_to_cpmm_virt_addr(phys_addr).as_mut_ptr()
    }

    unsafe fn free_slab(&mut self, slab_ptr: *mut u8, slab_size: usize, page_size: usize) {
        debug_assert!(!slab_ptr.is_null(), "Slab allocator tries to free null ptr");
        debug_assert!(
            slab_size != 0 && slab_size.is_power_of_two() && slab_size % page_size == 0,
            "Slab allocator tries to free invalid slab size"
        );
        let virt_addr = VirtAddr::from_ptr(slab_ptr);
        let phys_addr = super::virtual_memory_manager::virt_addr_from_cpmm_to_phys_addr(virt_addr);
        super::physical_memory_manager::free(phys_addr);
    }

    unsafe fn alloc_slab_info(&mut self) -> *mut SlabInfo {
        let slab_info_ptr = SLAB_INFO_CACHE
            .get()
            .expect("SlabInfo cache not set")
            .lock()
            .alloc();
        slab_info_ptr
    }

    unsafe fn free_slab_info(&mut self, slab_info_ptr: *mut SlabInfo) {
        debug_assert!(
            !slab_info_ptr.is_null(),
            "Slab allocator tries to free null ptr"
        );
        SLAB_INFO_CACHE
            .get()
            .expect("SlabInfo cache not set")
            .lock()
            .free(slab_info_ptr);
    }

    unsafe fn save_slab_info_ptr(&mut self, object_page_addr: usize, slab_info_ptr: *mut SlabInfo) {
        debug_assert!(
            object_page_addr != 0,
            "Slab allocator tries to save SlabInfo for zero page"
        );
        debug_assert!(
            !slab_info_ptr.is_null(),
            "Slab allocator tries to save SlabInfo with null ptr"
        );
        let virt_addr = VirtAddr::new(object_page_addr as u64);
        let phys_addr = super::virtual_memory_manager::virt_addr_from_cpmm_to_phys_addr(virt_addr);

        // OMG
        #[allow(static_mut_refs)]
        let slab_info_ptr_array_ref: &mut &mut [MaybeUninit<*mut SlabInfo>] = SLAB_INFO_PTRS
            .get_mut()
            .expect("SlabInfo ptr array not set");
        slab_info_ptr_array_ref[phys_addr.as_u64() as usize / PAGE_SIZE].write(slab_info_ptr);
    }

    unsafe fn get_slab_info_ptr(&mut self, object_page_addr: usize) -> *mut SlabInfo {
        debug_assert!(
            object_page_addr != 0,
            "Slab allocator tries to get SlabInfo for zero page"
        );

        let virt_addr = VirtAddr::new(object_page_addr as u64);
        let phys_addr = super::virtual_memory_manager::virt_addr_from_cpmm_to_phys_addr(virt_addr);

        #[allow(static_mut_refs)]
        let slab_info_ptr_array_ref: &&mut [MaybeUninit<*mut SlabInfo>] =
            SLAB_INFO_PTRS.get().expect("SlabInfo ptr array not set");
        slab_info_ptr_array_ref[phys_addr.as_u64() as usize / PAGE_SIZE].assume_init_read()
    }

    unsafe fn delete_slab_info_ptr(&mut self, page_addr: usize) {
        debug_assert!(
            page_addr != 0,
            "Slab allocator tries delete zero SlabInfo addr"
        );
        // Don't need to do anything
    }
}

struct SlabInfoCacheMemoryBackend;

impl MemoryBackend for SlabInfoCacheMemoryBackend {
    unsafe fn alloc_slab(&mut self, slab_size: usize, page_size: usize) -> *mut u8 {
        debug_assert!(
            slab_size != 0 && slab_size.is_power_of_two() && slab_size % page_size == 0,
            "SlabInfo allocator tries to allocate invalid slab size"
        );
        // Alloc physical frame with slab size
        let phys_addr = super::physical_memory_manager::alloc(
            &[
                MemoryZoneEnum::High,
                MemoryZoneEnum::IsaDma,
                MemoryZoneEnum::Dma32,
            ],
            slab_size,
        );
        if phys_addr.is_null() {
            return null_mut();
        }
        super::virtual_memory_manager::phys_addr_to_cpmm_virt_addr(phys_addr).as_mut_ptr()
    }

    unsafe fn free_slab(&mut self, slab_ptr: *mut u8, slab_size: usize, page_size: usize) {
        debug_assert!(!slab_ptr.is_null(), "Slab allocator tries to free null ptr");
        debug_assert!(
            slab_size != 0 && slab_size.is_power_of_two() && slab_size % page_size == 0,
            "SlabInfo allocator tries to free invalid slab size"
        );
        let virt_addr = VirtAddr::from_ptr(slab_ptr);
        let phys_addr = super::virtual_memory_manager::virt_addr_from_cpmm_to_phys_addr(virt_addr);
        super::physical_memory_manager::free(phys_addr);
    }

    unsafe fn alloc_slab_info(&mut self) -> *mut SlabInfo {
        unreachable!("SlabInfo allocator tries to allocate SlabInfo");
    }

    unsafe fn free_slab_info(&mut self, _slab_info_ptr: *mut SlabInfo) {
        unreachable!("SlabInfo allocator tries to free SlabInfo");
    }

    unsafe fn save_slab_info_ptr(
        &mut self,
        _object_page_addr: usize,
        _slab_info_ptr: *mut SlabInfo,
    ) {
        unreachable!("SlabInfo allocator tries to save SlabInfo");
    }

    unsafe fn get_slab_info_ptr(&mut self, _object_page_addr: usize) -> *mut SlabInfo {
        unreachable!("SlabInfo allocator tries to get SlabInfo");
    }

    unsafe fn delete_slab_info_ptr(&mut self, _page_addr: usize) {
        unreachable!("SlabInfo allocator tries to delete SlabInfo");
    }
}
