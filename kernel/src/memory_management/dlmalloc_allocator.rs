use core::alloc::{AllocError, Layout};
use core::ptr::NonNull;
use spin::{Mutex, Once};
use crate::memory_management::PAGE_SIZE;

static DLMALLOC_ALLOCATOR: Once<Mutex<DlmallocAllocator>> = Once::new();

struct DlmallocAllocator;

unsafe impl dlmalloc::Allocator for DlmallocAllocator {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        todo!()
    }

    fn remap(&self, ptr: *mut u8, oldsize: usize, newsize: usize, can_move: bool) -> *mut u8 {
        todo!()
    }

    fn free_part(&self, ptr: *mut u8, oldsize: usize, newsize: usize) -> bool {
        todo!()
    }

    fn free(&self, ptr: *mut u8, size: usize) -> bool {
        todo!()
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
