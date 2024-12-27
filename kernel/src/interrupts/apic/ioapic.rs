use super::super::idt::LOCAL_APIC_ISA_IRQ_VECTORS_RANGE;
use crate::memory_management::general_purpose_allocator::GeneralPurposeAllocator;
use acpi_lib::madt::{Madt, MadtEntry};
use acpi_lib::platform::interrupt::{Polarity, TriggerMode};
use acpi_lib::{AcpiTable, InterruptModel, ManagedSlice};
use bitfield::bitfield;
use core::ops::Add;
use spin::Once;
use tinyvec::ArrayVec;
use x86_64::{PhysAddr, VirtAddr};

static IO_APIC_PHYS_ADDR: Once<PhysAddr> = Once::new();

static IO_APIC_VIRT_ADDR: Once<VirtAddr> = Once::new();

pub fn init() {
    // Get MADT
    let acpi_tables_mutex_guard = crate::acpi::ACPI_TABLES.get().unwrap().lock();
    let madt = acpi_tables_mutex_guard
        .find_table::<Madt>()
        .expect("Failed to find MADT");
    madt.validate().expect("Failed to validate MADT");

    // Check platform info
    let platform_info = acpi_tables_mutex_guard
        .platform_info_in(GeneralPurposeAllocator)
        .expect("Failed to get platform info using ACPI tables");
    assert!(
        platform_info.processor_info.is_some(),
        "Processor info in platform info not found!"
    );

    // Check platform info and get IO APIC address
    let apic_info = match platform_info.interrupt_model {
        InterruptModel::Apic(ref apic_info) => {
            if apic_info.local_apic_address != super::BASE_PHYS_ADDR.as_u64() {
                panic!("Local APIC address in MADT differs from used!");
            }

            // I want to work with a single IO APIC and when GSI Base = 0.
            if apic_info.io_apics.len() == 0 {
                panic!("No IO APIC detected!");
            }
            if apic_info.io_apics.len() > 1 {
                unimplemented!("Multiple IO APIC's detected!");
            }
            if apic_info.io_apics[0].global_system_interrupt_base != 0 {
                unimplemented!("Global System Interrupt Base is not 0!");
            }

            // Get IO APIC address
            IO_APIC_PHYS_ADDR.call_once(|| PhysAddr::new(apic_info.io_apics[0].address as u64));
            IO_APIC_VIRT_ADDR.call_once(|| {
                crate::memory_management::virtual_memory_manager::phys_addr_to_cpmm_virt_addr(
                    *IO_APIC_PHYS_ADDR.get().unwrap(),
                )
            });

            apic_info
        }
        InterruptModel::Unknown => {
            panic!("Interrupt model in platform info is not APIC");
        }
        _ => unreachable!(),
    };

    // Configure IO APIC
    // Get number of entries in redirection table/number of pins
    let number_of_redirection_table_entries = ((read_ioapic_register(0x01) & 0xFF0000) >> 16) + 1;
    assert!(
        number_of_redirection_table_entries >= 24,
        "Number of redirection table entries in is less than 24, it looks like a bug"
    );
    assert!(
        number_of_redirection_table_entries <= 64,
        "Redirection table > 64, bug?"
    );

    // Fill redirection table
    // Fill with default value
    let mut redirection_table: ManagedSlice<RedirectionTableEntry, GeneralPurposeAllocator> =
        ManagedSlice::new_in(
            number_of_redirection_table_entries as usize,
            GeneralPurposeAllocator,
        )
        .expect("Failed to create slice");
    redirection_table.fill(RedirectionTableEntry(0));
    let bsp_apic_id = platform_info
        .processor_info
        .as_ref()
        .unwrap()
        .boot_processor
        .local_apic_id;
    for (i, entry) in redirection_table.iter_mut().enumerate() {
        let vector = i + *LOCAL_APIC_ISA_IRQ_VECTORS_RANGE.start() as usize;
        assert!(vector >= 0x10 && vector <= 0xFE);
        entry.set_vector(vector as u64);
        entry.set_delivery_mode(0); // Fixed
        entry.set_destination_mode(false); // Physical
        entry.set_interrupt_input_pin_polarity(false); // High Active by default
        entry.set_trigger_mode(false); // Edge-triggered by default
        entry.set_interrupt_mask(true); // Masked
        assert!(bsp_apic_id < 16); // 4 bytes
        entry.set_destination_field(bsp_apic_id as u64); // Destination - 4 bytes APIC ID for Physical Destination mode
    }

    // Unmask ISA IRQ's in redirection table
    for i in 0..16 {
        redirection_table[i].set_interrupt_mask(false);
    }

    // Modify the entries required by Interrupt Source Override table
    for interrupt_source_override in apic_info.interrupt_source_overrides.iter() {
        let isa_irq = interrupt_source_override.isa_source;
        let global_system_interrupt = interrupt_source_override.global_system_interrupt as usize;

        // ISA IRQ connected to Global System Interrupt (IO APIC pin)
        // Example: IRQ = 0 and GSI = 2, RT[2].vector must set to IRQ's 0 vector (LOCAL_APIC_ISA_IRQ_VECTORS_RANGE.start() + 0

        let vector = LOCAL_APIC_ISA_IRQ_VECTORS_RANGE.start() + isa_irq;
        assert!(
            LOCAL_APIC_ISA_IRQ_VECTORS_RANGE.contains(&vector),
            "Invalid vector calculated! Bug."
        );
        // Set vector of IRQ for IO APIC pin
        redirection_table[global_system_interrupt].set_vector(vector as u64);
        // Mask unused IO APIC pin
        redirection_table[isa_irq as usize].set_interrupt_mask(true);
        // Unmask used IO APIC pin (he could be masked if he himself had been reassigned earlier)
        redirection_table[global_system_interrupt].set_interrupt_mask(false);

        // Set Pin Polarity and Trigger Mode
        set_acpi_lib_pin_polarity_and_trigger_mode(
            interrupt_source_override.polarity,
            interrupt_source_override.trigger_mode,
            &mut redirection_table[global_system_interrupt],
        );
    }

    // Set NMI sources
    for nmi_source in apic_info.nmi_sources.iter() {
        // Unmask
        redirection_table[nmi_source.global_system_interrupt as usize].set_interrupt_mask(false);
        // Delivery Mode: NMI (vector ignored)
        redirection_table[nmi_source.global_system_interrupt as usize].set_delivery_mode(0b100);
        // Set Pin Polarity and Trigger Mode
        // Must be edge triggered
        set_acpi_lib_pin_polarity_and_trigger_mode(
            nmi_source.polarity,
            TriggerMode::Edge,
            &mut redirection_table[nmi_source.global_system_interrupt as usize],
        );
    }

    // All unused IO APIC pins masked now

    // Write redirection table
    assert_eq!(
        redirection_table.len(),
        number_of_redirection_table_entries as usize,
        "Redirection table len() incorrect! Bug."
    );
    for (index, entry) in redirection_table.iter().enumerate() {
        write_ioapic_redirection_table_entry(index as u8, entry);
    }
}

fn write_ioapic_register(offset: u8, val: u32) {
    let io_apic_virt_addr = IO_APIC_VIRT_ADDR
        .get()
        .expect("IO APIC VIRT ADDR is not set, bug");
    unsafe {
        // tell IOREGSEL where we want to write to
        io_apic_virt_addr
            .as_mut_ptr::<u32>()
            .write_volatile(offset as u32);
        // write the value to IOWIN
        io_apic_virt_addr
            .add(0x10)
            .as_mut_ptr::<u32>()
            .write_volatile(val);
    }
}

fn read_ioapic_register(offset: u8) -> u32 {
    let io_apic_virt_addr = IO_APIC_VIRT_ADDR
        .get()
        .expect("IO APIC VIRT ADDR is not set, bug");
    unsafe {
        // tell IOREGSEL where we want to read from
        io_apic_virt_addr
            .as_mut_ptr::<u32>()
            .write_volatile(offset as u32);
        // return the data from IOWIN
        io_apic_virt_addr
            .add(0x10)
            .as_mut_ptr::<u32>()
            .read_volatile()
    }
}

fn write_ioapic_redirection_table_entry(
    index: u8,
    redirection_table_entry: &RedirectionTableEntry,
) {
    let io_apic_virt_addr = IO_APIC_VIRT_ADDR
        .get()
        .expect("IO APIC VIRT ADDR is not set, bug");
    let offset_low = 0x10 + 2 * index;
    let offset_high = offset_low + 1;
    let low = redirection_table_entry.0 as u32;
    let high = (redirection_table_entry.0 >> 32) as u32;
    unsafe {
        write_ioapic_register(offset_low, low);
        write_ioapic_register(offset_high, high);
    }
}

bitfield! {
    #[derive(Copy, Clone)]
    struct RedirectionTableEntry(u64);
    vector, set_vector: 7, 0;
    delivery_mode, set_delivery_mode: 10, 8;
    destination_mode, set_destination_mode: 11;
    delivery_status, set_delivery_status: 12;
    interrupt_input_pin_polarity, set_interrupt_input_pin_polarity: 13;
    remote_irr, set_remote_irr: 14;
    trigger_mode, set_trigger_mode: 15;
    interrupt_mask, set_interrupt_mask: 16;
    destination_field, set_destination_field: 63, 56;
}

fn set_acpi_lib_pin_polarity_and_trigger_mode(
    polarity: Polarity,
    trigger_mode: TriggerMode,
    redirection_table_entry: &mut RedirectionTableEntry,
) {
    // Set Pin Polarity and Trigger Mode
    match polarity {
        Polarity::ActiveHigh => redirection_table_entry.set_interrupt_input_pin_polarity(false),
        Polarity::ActiveLow => redirection_table_entry.set_interrupt_input_pin_polarity(true),
        Polarity::SameAsBus => redirection_table_entry.set_interrupt_input_pin_polarity(false),
    }
    match trigger_mode {
        TriggerMode::Edge => redirection_table_entry.set_trigger_mode(false),
        TriggerMode::Level => redirection_table_entry.set_trigger_mode(true),
        TriggerMode::SameAsBus => redirection_table_entry.set_trigger_mode(false),
    }
}
