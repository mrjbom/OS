use x86_64::instructions::tlb;
use x86_64::structures::paging::page_table::PageTableLevel;
use x86_64::structures::paging::{PageTable, PageTableFlags};
use x86_64::{PhysAddr, VirtAddr};

// TODO: Idea: Add different wrapper types for virtual addresses belonging to different areas,
// this is due to the fact that their conversion to physical addresses may differ.
// A virtual address from a Complete Physical Memory Mapping area can be easily converted to a physical address,
// but a virtual address from a user space or a virtual address from a Virtual Memory Allocations area should be treated differently.

/// Complete Physical Memory Mapping offset in virtual memory
///
/// doc/virtual_memory_layout.txt
pub const PHYSICAL_MEMORY_MAPPING_OFFSET: u64 = 0xFFFF_A000_0000_0000;

/// Setting up some virtual memory things
pub fn init() {
    // Unmap all pages in userspace (lower half)
    // https://github.com/rust-osdev/bootloader/issues/470
    // Bootloader left some stuff in there, such as context switch function and GDT. These things must be unmapped.
    // I left first 128 TB for userspace
    // First 128 TB represended by first 256 entries of PML4
    let (pml4, _) = x86_64::registers::control::Cr3::read();
    assert!(!pml4.start_address().is_null());
    let pml4 = phys_addr_to_cpmm_virt_addr(pml4.start_address());
    let pml4 = pml4.as_mut_ptr::<PageTable>();
    // Unmap first 128 TB
    for i in 0..256 {
        unsafe {
            (*pml4)[i].set_unused();
        }
    }
    tlb::flush_all();
}

/// Converts physical address to virtual address in Complete Physical Memory Mapping area
///
/// Adds PHYSICAL_MEMORY_MAPPING_OFFSET to physical address
#[inline]
pub const fn phys_addr_to_cpmm_virt_addr(phys_addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys_addr.as_u64() + PHYSICAL_MEMORY_MAPPING_OFFSET)
}

/// Converts virtual address from Complete Physical Memory Mapping area to physical address
///
/// Subs PHYSICAL_MEMORY_MAPPING_OFFSET from virtual address
#[inline]
pub const fn virt_addr_from_cpmm_to_phys_addr(virt_addr: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt_addr.as_u64() - PHYSICAL_MEMORY_MAPPING_OFFSET)
}

/// Sets flags to value in selected page table level by virtual addr
///
/// If the selected page table level does not exist due to huge (2MB or 1GB) page using, the flags will be applied to the existing level above.
///
/// Doesn't flush TLB
pub fn set_flags_in_page_table(
    virt_addr: VirtAddr,
    page_table_level: PageTableLevel,
    page_table_flags: PageTableFlags,
    value: bool,
) {
    let mut current_level = PageTableLevel::Four;
    let mut page_table_phys_addr = x86_64::registers::control::Cr3::read().0.start_address();
    loop {
        let page_table_virt_addr = phys_addr_to_cpmm_virt_addr(page_table_phys_addr);
        let page_table = page_table_virt_addr.as_mut_ptr::<PageTable>();
        debug_assert!(!page_table.is_null(), "Page table null ptr");
        debug_assert!(page_table.is_aligned(), "Not aligned page table address");

        unsafe {
            let index = virt_addr.page_table_index(current_level);
            if current_level == page_table_level
                || (*page_table)[index]
                    .flags()
                    .contains(PageTableFlags::HUGE_PAGE)
            {
                let mut flags = (*page_table)[index].flags();
                flags.set(page_table_flags, value);
                (*page_table)[index].set_flags(flags);
                return;
            }
            current_level = current_level.next_lower_level().unwrap();
            page_table_phys_addr = (*page_table)[index].addr();
        }
    }
}
