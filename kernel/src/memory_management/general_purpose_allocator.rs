use crate::memory_management::physical_memory_manager::MemoryZoneEnum;
use crate::memory_management::PAGE_SIZE;
use core::alloc::{AllocError, Layout};
use core::ptr::{null_mut, NonNull};
use spin::{Mutex, Once};
use x86_64::{PhysAddr, VirtAddr};

/// "System" allocator required for Dlmalloc allocator
///
/// Wrapper over buddy allocator
struct DlmallocSystemAllocator;

unsafe impl dlmalloc::Allocator for DlmallocSystemAllocator {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        if !(size >= PAGE_SIZE && size.is_power_of_two()) {
            unimplemented!(
                "dlmalloc tries to allocate a memory size not suitable for buddy allocator: {size}"
            );
        }

        let phys_addr = unsafe {
            super::physical_memory_manager::alloc(
                &[
                    MemoryZoneEnum::High,
                    MemoryZoneEnum::Dma32,
                    MemoryZoneEnum::IsaDma,
                ],
                size,
            )
        };
        if phys_addr.is_null() {
            return (null_mut(), 0, 0);
        }
        let virt_addr = super::virtual_memory_manager::phys_addr_to_cpmm_virt_addr(phys_addr);
        (virt_addr.as_mut_ptr(), size, 0)
    }

    fn remap(&self, ptr: *mut u8, oldsize: usize, newsize: usize, can_move: bool) -> *mut u8 {
        debug_assert!(!ptr.is_null(), "dmalloc tries to remap null ptr");
        if !(oldsize >= PAGE_SIZE && oldsize.is_power_of_two()) {
            unimplemented!("dlmalloc tries to remap a memory with oldsize not suitable for buddy allocator: {oldsize}");
        }
        if !(newsize >= PAGE_SIZE && newsize.is_power_of_two()) {
            unimplemented!("dlmalloc tries to remap a memory with newsize not suitable for buddy allocator: {newsize}");
        }

        if can_move {
            let virt_addr = VirtAddr::from_ptr(ptr);
            let phys_addr =
                super::virtual_memory_manager::virt_addr_from_cpmm_to_phys_addr(virt_addr);
            unsafe {
                let new_phys_addr =
                    super::physical_memory_manager::realloc(phys_addr, newsize, true);
                if new_phys_addr.is_null() {
                    return null_mut();
                }
                let new_phys_addr = PhysAddr::new(new_phys_addr as u64);
                let new_virt_addr =
                    super::virtual_memory_manager::phys_addr_to_cpmm_virt_addr(new_phys_addr);
                new_virt_addr.as_mut_ptr()
            }
        } else {
            null_mut()
        }
    }

    fn free_part(&self, ptr: *mut u8, oldsize: usize, newsize: usize) -> bool {
        unreachable!("dlmalloc should not call this function");
    }

    fn free(&self, ptr: *mut u8, size: usize) -> bool {
        debug_assert!(!ptr.is_null(), "dmalloc tries to free null ptr");
        if !(size >= PAGE_SIZE && size.is_power_of_two()) {
            unimplemented!("dlmalloc tries to free a memory with size not suitable for buddy allocator: {size}");
        }

        let virt_addr = VirtAddr::from_ptr(ptr);
        let phys_addr = super::virtual_memory_manager::virt_addr_from_cpmm_to_phys_addr(virt_addr);
        unsafe {
            super::physical_memory_manager::free(phys_addr);
        }

        true
    }

    fn can_release_part(&self, _flags: u32) -> bool {
        false
    }

    fn allocates_zeros(&self) -> bool {
        false
    }

    fn page_size(&self) -> usize {
        PAGE_SIZE
    }
}

/// Allocator that implements the Allocator trait and can be used as a general-purpose allocator, mainly for libraries that require it
///
/// A SLAB allocator should be used for frequent and basic selection of kernel objects of the same size.
///
/// Uses dlmalloc.
pub struct GeneralPurposeAllocator;

unsafe impl core::alloc::Allocator for GeneralPurposeAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        todo!()
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        todo!()
    }
}
