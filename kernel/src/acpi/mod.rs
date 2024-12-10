use crate::memory_management::virtual_memory_manager::PHYSICAL_MEMORY_MAPPING_OFFSET;
use crate::memory_management::PAGE_SIZE;
use acpi::{AcpiTables, PhysicalMapping};
use bootloader_api::BootInfo;
use core::cell::OnceCell;
use core::ptr::NonNull;
use spin::Mutex;
use x86_64::VirtAddr;

pub static ACPI_TABLES: Mutex<OnceCell<AcpiTables<BaseAcpiHandler>>> = Mutex::new(OnceCell::new());

/// Gets ACPI tables
pub fn init(boot_info: &BootInfo) {
    // Check RSDP address
    let rsdp_phys_addr = boot_info
        .rsdp_addr
        .into_option()
        .expect("Bootloader could not find RSDP");

    // Validate RSDP
    let rsdp =
        VirtAddr::new(rsdp_phys_addr + PHYSICAL_MEMORY_MAPPING_OFFSET).as_ptr::<acpi::rsdp::Rsdp>();
    unsafe {
        (*rsdp).validate().expect("Invalid RSDP!");
    }

    let acpi_tables = unsafe {
        AcpiTables::from_rsdp(BaseAcpiHandler, rsdp_phys_addr as usize)
            .expect("Failed to get ACPI tables")
    };

    ACPI_TABLES
        .lock()
        .set(acpi_tables)
        .expect_err("ACPI_TABLES already sets");
}

#[derive(Clone)]
struct BaseAcpiHandler;

impl acpi::AcpiHandler for BaseAcpiHandler {
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

        let virtual_address =
            VirtAddr::new(physical_address as u64 + PHYSICAL_MEMORY_MAPPING_OFFSET);

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
