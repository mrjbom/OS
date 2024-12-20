use acpi::madt::{Madt, MadtEntry};
use acpi::AcpiTable;

pub fn init() {
    // Get MADT
    let acpi_tables_mutex_guard = crate::acpi::ACPI_TABLES.lock();
    log::debug!("Find MADT");
    let madt = acpi_tables_mutex_guard
        .get()
        .unwrap()
        .find_table::<Madt>()
        .expect("Failed to find MADT");
    madt.validate().expect("Failed to validate MADT");

    // Get IO APIC address from MADT
    for madt_entry in madt.entries() {
        if let MadtEntry::IoApic(io_apic_entry) = madt_entry {
            log::debug!("IO APIC entry detected: {io_apic_entry:?}");
        }
    }
}
