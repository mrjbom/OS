use crate::memory_management::virtual_memory_manager;
use crate::memory_management::PAGE_SIZE;
use acpi_lib::{AcpiTables, PhysicalMapping};
use bootloader_api::BootInfo;
use core::ptr::NonNull;
use spin::{Mutex, Once};
use x86_64::{PhysAddr, VirtAddr};

pub static ACPI_TABLES: Once<Mutex<AcpiTables<BaseAcpiHandler>>> = Once::new();

/// Gets ACPI tables
pub fn init(boot_info: &BootInfo) {
    // Get RSDP address
    let rsdp_phys_addr = PhysAddr::new(
        boot_info
            .rsdp_addr
            .into_option()
            .expect("Bootloader could not find RSDP"),
    );

    // Validate RSDP
    let rsdp = virtual_memory_manager::phys_addr_to_cpmm_virt_addr(rsdp_phys_addr)
        .as_ptr::<acpi_lib::rsdp::Rsdp>();
    unsafe {
        (*rsdp).validate().expect("Invalid RSDP!");
    }

    // Create ACPI tables
    let acpi_tables = unsafe {
        AcpiTables::from_rsdp(BaseAcpiHandler, rsdp_phys_addr.as_u64() as usize)
            .expect("Failed to get ACPI tables")
    };

    ACPI_TABLES.call_once(|| Mutex::new(acpi_tables));
}

#[derive(Debug, Clone)]
pub struct BaseAcpiHandler;

impl acpi_lib::AcpiHandler for BaseAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        _size: usize,
    ) -> PhysicalMapping<Self, T> {
        // We just need to return the virtual address from Complete Physical Memory Mapping region

        let physical_region_start = x86_64::align_down(physical_address as u64, PAGE_SIZE as u64);
        let physical_region_size = {
            let size = physical_address as u64 - physical_region_start;
            if size == 0 {
                PAGE_SIZE
            } else {
                x86_64::align_up(size, PAGE_SIZE as u64) as usize
            }
        };
        debug_assert_eq!(physical_region_start as usize % PAGE_SIZE, 0);
        debug_assert!(physical_region_size >= PAGE_SIZE);

        let virtual_address = virtual_memory_manager::phys_addr_to_cpmm_virt_addr(PhysAddr::new(
            physical_address as u64,
        ));

        PhysicalMapping::new(
            physical_address,
            NonNull::new(virtual_address.as_mut_ptr::<T>()).unwrap(),
            size_of::<T>(),
            physical_region_size,
            self.clone(),
        )
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // There is no need to do anything
    }
}
