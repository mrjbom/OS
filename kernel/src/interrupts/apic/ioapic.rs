use crate::memory_management::general_purpose_allocator::GeneralPurposeAllocator;
use acpi_lib::madt::{Madt, MadtEntry};
use acpi_lib::AcpiTable;

pub fn init() {
    // Get MADT
    let acpi_tables_mutex_guard = crate::acpi::ACPI_TABLES.get().unwrap().lock();
    log::debug!("Find MADT");
    let madt = acpi_tables_mutex_guard
        .find_table::<Madt>()
        .expect("Failed to find MADT");
    madt.validate().expect("Failed to validate MADT");

    // Get IO APIC address from MADT
    for madt_entry in madt.entries() {
        if let MadtEntry::IoApic(io_apic_entry) = madt_entry {
            log::debug!("IO APIC entry detected: {io_apic_entry:?}");
        }
    }

    let platform_info = acpi_tables_mutex_guard
        .platform_info_in(GeneralPurposeAllocator)
        .expect("Failed to get platfor info from ACPI tables");
    log::debug!("Platform info: {platform_info:#?}");
}
