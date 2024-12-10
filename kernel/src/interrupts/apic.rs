use crate::memory_management::virtual_memory_manager::{self, PHYSICAL_MEMORY_MAPPING_OFFSET};
use crate::memory_management::PAGE_SIZE;
use acpi::PhysicalMapping;
use bootloader_api::BootInfo;
use core::ptr::NonNull;
use raw_cpuid::CpuId;
use x86_64::instructions::tlb;
use x86_64::structures::paging::page_table::PageTableLevel;
use x86_64::structures::paging::PageTableFlags;
use x86_64::{PhysAddr, VirtAddr};

/// By default, local APIC base, APIC registers are placed on this physical page
const LOCAL_APIC_BASE_ADDR: PhysAddr = PhysAddr::new(0xFEE00000);

/// Virtual address of local APIC base in Complete Physical Memory Mapping
///
/// ## Must be mapped without caching
const LOCAL_APIC_BASE_MAPPED_ADDR: VirtAddr =
    virtual_memory_manager::phys_addr_to_cpmm_virt_addr(LOCAL_APIC_BASE_ADDR);

// Registers
/// 0xF0	Spurious-Interrupt Vector Register
const SPURIOUS_INTERRUPT_VECTOR_REGISTER: *mut u32 =
    (LOCAL_APIC_BASE_MAPPED_ADDR.as_u64() + 0xF0) as *mut u32;
/// 0x320   LVT Timer Register
const LVT_TIMER_REGISTER: *mut u32 = (LOCAL_APIC_BASE_MAPPED_ADDR.as_u64() + 0x320) as *mut u32;
/// 0x350   LVT LINT0 Register
const LVT_LINT0_REGISTER: *mut u32 = (LOCAL_APIC_BASE_MAPPED_ADDR.as_u64() + 0x350) as *mut u32;
/// 0x360   LVT LINT1 Register
const LVT_LINT1_REGISTER: *mut u32 = (LOCAL_APIC_BASE_MAPPED_ADDR.as_u64() + 0x360) as *mut u32;
/// 0x370   LVT Error Register
const LVT_ERROR_REGISTER: *mut u32 = (LOCAL_APIC_BASE_MAPPED_ADDR.as_u64() + 0x370) as *mut u32;

pub fn init(boot_info: &BootInfo) {
    // Check APIC support
    log::info!("Checking APIC support");
    let cpuid = CpuId::new();
    let cpuid_feature_info = cpuid
        .get_feature_info()
        .expect("Failed to get CPUID features!");
    if !cpuid_feature_info.has_apic() {
        panic!("APIC not supported");
    }

    // Check ACPI RSDP address
    log::info!("Checking ACPI RSDP address");
    boot_info
        .rsdp_addr
        .into_option()
        .expect("ACPI RSDP address not detected by bootloader!");

    // Validate RSDP
    let rsdp =
        VirtAddr::new(boot_info.rsdp_addr.into_option().unwrap() + PHYSICAL_MEMORY_MAPPING_OFFSET)
            .as_ptr::<acpi::rsdp::Rsdp>();
    unsafe {
        (*rsdp).validate().expect("Invalid RSDP!");
    }

    // Check APIC base address from MSR
    let ia32_apic_base_msr = unsafe { x86_64::registers::model_specific::Msr::new(0x1B).read() };
    let apic_base_page_phys_addr_from_msr =
        x86_64::align_down(ia32_apic_base_msr, PAGE_SIZE as u64);
    assert_eq!(
        apic_base_page_phys_addr_from_msr,
        LOCAL_APIC_BASE_ADDR.as_u64(),
        "The APIC base address is not on the default page!"
    );

    // Make APIC base mapping page uncacheable
    // osdev wiki: Section 11.4.1 of 3rd volume of Intel SDM recommends mapping the base address page as strong uncacheable for correct APIC operation.
    // My SDM (May 2020) in 10.4.1 says:
    // APIC registers are memory-mapped to a 4-KByte region of the processor’s physical
    // address space with an initial starting address of FEE00000H. For correct APIC operation, this address space must
    // be mapped to an area of memory that has been designated as strong uncacheable (UC)
    virtual_memory_manager::set_flags_in_page_table(
        LOCAL_APIC_BASE_MAPPED_ADDR,
        PageTableLevel::One,
        PageTableFlags::NO_CACHE | PageTableFlags::WRITE_THROUGH,
        true,
    );
    tlb::flush(LOCAL_APIC_BASE_MAPPED_ADDR);

    // Disable interrupts
    x86_64::instructions::interrupts::disable();

    // Disable PIC
    // # https://wiki.osdev.org/8259_PIC#Disabling
    #[allow(static_mut_refs)]
    unsafe {
        super::pic::PICS.disable()
    };

    fill_lvt_timer_register();
    fill_lvt_lint0_register();
    fill_lvt_lint1_register();
    fill_lvt_error_register();

    // Enable APIC
    fill_spurious_interrupt_vector_register();
}

fn fill_lvt_timer_register() {
    let mut register_value: u32 = 0;
    // Vector               0-7 = IDT vector
    // Delivery Status      12 = 0 - (Read Only)
    // Mask                 16 = 0 - Unmasked
    // Timer Periodic Mode  17 = 0 - Fired only once
    register_value |= super::ACPI_TIMER_IDT_VECTOR as u32;
    unsafe {
        LVT_TIMER_REGISTER.write_volatile(register_value);
    }
}

fn fill_lvt_lint0_register() {
    let mut register_value: u32 = 0;
    // Vector                           0-7 = IDT vector
    // Delivery Mode                    8-10 = 0 - Fixed
    // Delivery Status                  12 = 0 - (Read Only)
    // Interrupt Input Pin Polarity     13 = 0 - High
    // Remote IRR                       14 = 0 - (Read Only)
    // Trigger Mode                     15 = 0 - Edge Triggered
    // Mask                             16 = 0 - Unmasked
    register_value |= super::ACPI_LINT0_IDT_VECTOR as u32;
    unsafe {
        LVT_LINT0_REGISTER.write_volatile(register_value);
    }
}

fn fill_lvt_lint1_register() {
    let mut register_value: u32 = 0;
    // Vector                           0-7 = IDT vector
    // Delivery Mode                    8-10 = 0 - Fixed
    // Delivery Status                  12 = 0 - (Read Only)
    // Interrupt Input Pin Polarity     13 = 0 - High
    // Remote IRR                       14 = 0 - (Read Only)
    // Trigger Mode                     15 = 0 - Always Edge Triggered (Must be Edge Triggered for LINT1)
    // Mask                             16 = 0 - Unmasked
    register_value |= super::ACPI_LINT1_IDT_VECTOR as u32;
    unsafe {
        LVT_LINT1_REGISTER.write_volatile(register_value);
    }
}

fn fill_lvt_error_register() {
    let mut register_value: u32 = 0;
    // Vector                           0-7 = IDT vector
    // Delivery Status                  12 = 0 - (Read Only)
    // Mask                             16 = 0 - Unmasked
    register_value |= super::ACPI_ERROR_IDT_VECTOR as u32;
    unsafe {
        LVT_ERROR_REGISTER.write_volatile(register_value);
    }
}

/// Enables interrupts
fn fill_spurious_interrupt_vector_register() {
    let mut register_value: u32 = 0;
    // Vector                           0-7 = IDT vector (0-3 always 1111)
    // APIC Software Enable             8 = 1 Enabled
    // Focus Processing Checking        9 = 0 Disabled
    // EOI-Broadcast Suppression        12 = 0 Disabled
    assert_eq!(
        super::ACPI_SPURIOUS_IDT_VECTOR & 0b00001111,
        0b00001111,
        "Invalid spurious vector number"
    );
    register_value |= super::ACPI_SPURIOUS_IDT_VECTOR as u32;
    unsafe {
        SPURIOUS_INTERRUPT_VECTOR_REGISTER.write_volatile(register_value);
    }
}

#[derive(Clone)]
struct MyAcpiHandler;

impl acpi::AcpiHandler for MyAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
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
        debug_assert!(size >= PAGE_SIZE);

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
