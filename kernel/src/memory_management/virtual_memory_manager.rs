use x86_64::{PhysAddr, VirtAddr};

/// doc/virtual_memory_layout.txt
/// Complete Physical Memory Mapping offset in virtual memory
pub const PHYSICAL_MEMORY_MAPPING_OFFSET: u64 = 0xFFFF_A000_0000_0000;

/// Converts physical address to virtual address in Complete Physical Memory Mapping area
///
/// Adds PHYSICAL_MEMORY_MAPPING_OFFSET to physical address
#[inline]
pub fn phys_addr_to_cpmm_virt_addr(phys_addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys_addr.as_u64() + PHYSICAL_MEMORY_MAPPING_OFFSET)
}

/// Converts virtual address from Complete Physical Memory Mapping area to physical address
///
/// Subs PHYSICAL_MEMORY_MAPPING_OFFSET from virtual address
#[inline]
pub fn virt_addr_from_cpmm_to_phys_addr(virt_addr: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt_addr.as_u64() - PHYSICAL_MEMORY_MAPPING_OFFSET)
}
