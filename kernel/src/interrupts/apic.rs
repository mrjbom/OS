mod ioapic;

use crate::acpi::{ACPI_TABLES, PLATFORM_INFO};
use crate::memory_management::general_purpose_allocator::GeneralPurposeAllocator;
use crate::memory_management::virtual_memory_manager;
use crate::memory_management::PAGE_SIZE;
use acpi_lib::platform::interrupt::{LocalInterruptLine, NmiProcessor};
use acpi_lib::InterruptModel;
use bitfield::bitfield;
use raw_cpuid::CpuId;
use x86_64::instructions::tlb;
use x86_64::structures::paging::page_table::PageTableLevel;
use x86_64::structures::paging::PageTableFlags;
use x86_64::{PhysAddr, VirtAddr};

static LOCAL_APIC_VERSION: spin::Once<LocalApicVersion> = spin::Once::new();

/// Defined in Local APIC Version Register
#[derive(Debug, PartialEq)]
enum LocalApicVersion {
    /// 82489DX discrete APIC.
    Descrete,
    /// Integrated APIC.
    Integrated,
}

/// By default, local APIC base, APIC registers are placed on this physical page
const BASE_PHYS_ADDR: PhysAddr = PhysAddr::new(0xFEE00000);

/// Virtual address of local APIC base in Complete Physical Memory Mapping
///
/// ## Must be mapped without caching
const BASE_VIRT_ADDR: VirtAddr =
    virtual_memory_manager::phys_addr_to_cpmm_virt_addr(BASE_PHYS_ADDR);

// Registers
/// 0x30    Local APIC Version Register
const VERSION_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0x30) as *mut u32;

/// 0xB0    End Of Interrupt Register
const EOI_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0xB0) as *mut u32;

/// 0xF0    Spurious-Interrupt Vector Register
const SPURIOUS_INTERRUPT_VECTOR_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0xF0) as *mut u32;

/// 0x320   LVT Timer Register
const LVT_TIMER_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0x320) as *mut u32;

/// 0x350   LVT LINT0 Register
const LVT_LINT0_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0x350) as *mut u32;

/// 0x360   LVT LINT1 Register
const LVT_LINT1_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0x360) as *mut u32;

/// 0x370   LVT Error Register
const LVT_ERROR_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0x370) as *mut u32;

/// 0x380   Initial Count Register
const INITIAL_COUNT_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0x380) as *mut u32;

/// 0x390   Current Count Register
const CURRENT_COUNT_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0x390) as *mut u32;

/// 0x3E0   Divide Configuration Register
const DIVIDE_CONFIGURATION_REGISTER: *mut u32 = (BASE_VIRT_ADDR.as_u64() + 0x3E0) as *mut u32;

/// Inits Local APIC for this CPU (BSP)
pub fn init() {
    // Disable interrupts
    x86_64::instructions::interrupts::disable();

    // Check APIC support
    let cpuid = CpuId::new();
    let cpuid_feature_info = cpuid
        .get_feature_info()
        .expect("Failed to get CPUID features!");
    if !cpuid_feature_info.has_apic() {
        panic!("APIC not supported");
    }

    // Check APIC base address from MSR (Intel and AMD supported)
    let ia32_apic_base_msr = unsafe { x86_64::registers::model_specific::Msr::new(0x1B).read() };
    let apic_base_page_phys_addr_from_msr =
        x86_64::align_down(ia32_apic_base_msr, PAGE_SIZE as u64);
    assert_eq!(
        apic_base_page_phys_addr_from_msr,
        BASE_PHYS_ADDR.as_u64(),
        "The APIC base address is not on the default page!"
    );

    // Make APIC base mapping page uncacheable
    // osdev wiki: Section 11.4.1 of 3rd volume of Intel SDM recommends mapping the base address page as strong uncacheable for correct APIC operation.
    // My SDM (May 2020) in 10.4.1 says:
    // APIC registers are memory-mapped to a 4-KByte region of the processor’s physical
    // address space with an initial starting address of FEE00000H. For correct APIC operation, this address space must
    // be mapped to an area of memory that has been designated as strong uncacheable (UC)
    virtual_memory_manager::set_flags_in_page_table(
        BASE_VIRT_ADDR,
        PageTableLevel::One,
        PageTableFlags::NO_CACHE | PageTableFlags::WRITE_THROUGH,
        true,
    );
    tlb::flush(BASE_VIRT_ADDR);

    // Determine whether the 82489DX is a discrete APIC or an Integrated APIC using the Local APIC Version Register
    // Version bits 0-7:
    // 0 -           82489DX Discrete
    // 0x10 - 0x15 - Integrated
    let local_apic_version_register_value = unsafe { *VERSION_REGISTER };
    let version: u8 = local_apic_version_register_value as u8;
    match version {
        0 => LOCAL_APIC_VERSION.call_once(|| LocalApicVersion::Descrete),
        0x10..=0x15 => LOCAL_APIC_VERSION.call_once(|| LocalApicVersion::Integrated),
        _ => unreachable!("Reserved value"),
    };

    // APIC enabled by default, but interrupts masked, need set vectors and unmask
    // Fill LVT registers (set and unmask vectors)
    let bsp_uid = PLATFORM_INFO
        .get()
        .expect("Failed to get PlatformInfo")
        .processor_info
        .as_ref()
        .unwrap()
        .boot_processor
        .processor_uid;
    fill_spurious_interrupt_vector_register();
    fill_lvt_lint0_register(bsp_uid);
    fill_lvt_lint1_register(bsp_uid);
    fill_lvt_error_register();

    // Configure IO APIC for Legacy ISA IRQ's
    ioapic::init();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();
}

/// Set and unmasks APIC Timer interrupt vector <br>
/// Vector               0-7     = IDT vector <br>
/// Delivery Status      12      = 0 - (Read Only) <br>
/// Mask                 16      = 0 - Unmasked <br>
/// Timer Periodic Mode  17-18   = 00 - Fired only once <br>
fn fill_lvt_timer_register() {
    let mut register_value = LvtRegister(0);
    register_value.set_vector(super::idt::LOCAL_APIC_TIMER_IDT_VECTOR as u32);

    unsafe {
        LVT_TIMER_REGISTER.write_volatile(register_value.0);
    }
}

/// Set and unmasks APIC LINT0 interrupt vector <br>
/// Vector                           0-7 = IDT vector <br>
/// Delivery Mode                    8-10 = 000 - Fixed <br>
/// Delivery Status                  12 = 0 - (Read Only) <br>
/// Interrupt Input Pin Polarity     13 = 0 - High <br>
/// Remote IRR                       14 = 0 - (Read Only) <br>
/// Trigger Mode                     15 = 0 - Edge Triggered <br>
/// Mask                             16 = 0 - Unmasked <br>
fn fill_lvt_lint0_register(processor_uid: u32) {
    let mut register_value = LvtRegister(0);
    register_value.set_vector(super::idt::LOCAL_APIC_LINT0_IDT_VECTOR as u32);

    set_nmi_if_needed(
        &mut register_value,
        LocalInterruptLine::Lint0,
        processor_uid,
    );

    unsafe {
        LVT_LINT0_REGISTER.write_volatile(register_value.0);
    }
}

/// Set and unmasks APIC LINT1 interrupt vector <br>
/// Vector                           0-7 = IDT vector <br>
/// Delivery Mode                    8-10 = 000 - Fixed <br>
/// Delivery Status                  12 = 0 - (Read Only) <br>
/// Interrupt Input Pin Polarity     13 = 0 - High <br>
/// Remote IRR                       14 = 0 - (Read Only) <br>
/// Trigger Mode                     15 = 0 - Always Edge Triggered (Must be Edge Triggered for LINT1) <br>
/// Mask                             16 = 0 - Unmasked <br>
fn fill_lvt_lint1_register(processor_uid: u32) {
    let mut register_value = LvtRegister(0);
    register_value.set_vector(super::idt::LOCAL_APIC_LINT1_IDT_VECTOR as u32);

    set_nmi_if_needed(
        &mut register_value,
        LocalInterruptLine::Lint1,
        processor_uid,
    );

    unsafe {
        LVT_LINT1_REGISTER.write_volatile(register_value.0);
    }
}

/// Sets NMI delivery mode for LINT# if it's required by ACPI table
fn set_nmi_if_needed(
    lvt_register: &mut LvtRegister,
    local_interrupt_line: LocalInterruptLine,
    processor_uid: u32,
) {
    let platform_info = PLATFORM_INFO.get().unwrap();

    if let InterruptModel::Apic(ref apic_info) = platform_info.interrupt_model {
        for nmi_line in apic_info.local_apic_nmi_lines.iter() {
            if nmi_line.line == local_interrupt_line {
                let mut need_set_nmi = false;
                match nmi_line.processor {
                    NmiProcessor::All => need_set_nmi = true,
                    NmiProcessor::ProcessorUid(uid) => {
                        if uid == processor_uid {
                            need_set_nmi = true;
                        }
                    }
                }
                if need_set_nmi {
                    // Delivery mode: NMI
                    lvt_register.set_delivery_mode(0b100);
                }
            }
        }
    }
}

/// Set and unmasks APIC Error interrupt vector <br>
/// Vector                           0-7 = IDT vector <br>
/// Delivery Status                  12 = 0 - (Read Only) <br>
/// Mask                             16 = 0 - Unmasked <br>
fn fill_lvt_error_register() {
    let mut register_value = LvtRegister(0);
    register_value.set_vector(super::idt::LOCAL_APIC_ERROR_IDT_VECTOR as u32);

    unsafe {
        LVT_ERROR_REGISTER.write_volatile(register_value.0);
    }
}

/// Sets spurious interrupt vector interrupts, enables APIC interrupts (Enabled by Default) <br>
/// Vector                           0-7 = IDT vector (0-3 always 1111) <br>
/// APIC Software Enable             8 = 1 Enabled (ENABLED BY DEFAULT) <br>
/// Focus Processing Checking        9 = 0 Disabled <br>
/// EOI-Broadcast Suppression        12 = 0 Disabled <br>
fn fill_spurious_interrupt_vector_register() {
    assert_eq!(
        super::idt::LOCAL_APIC_SPURIOUS_IDT_VECTOR & 0b00001111,
        0b00001111,
        "Invalid spurious vector number"
    );
    let mut register_value: u32 = 0;
    register_value |= super::idt::LOCAL_APIC_SPURIOUS_IDT_VECTOR as u32;
    // Set 8 bit (Enabled by default!)
    register_value |= 1 << 8;

    unsafe {
        SPURIOUS_INTERRUPT_VECTOR_REGISTER.write_volatile(register_value);
    }
}

/// ## Don't use for Spurious Interrupt
#[inline]
pub fn send_eoi() {
    unsafe {
        EOI_REGISTER.write_volatile(0);
    }
}

bitfield! {
    struct LvtRegister(u32);
    vector, set_vector: 7, 0;
    delivery_mode, set_delivery_mode: 10, 8;
    delivery_status, set_delivery_status: 12;
    interrupt_input_pin_polarity, set_interrupt_input_pin_polarity: 13;
    remote_irr, set_remote_irr: 14;
    trigger_mode, set_trigger_mode: 15;
    mask, set_mask: 16;
    timer_mode, set_timer_mode: 18, 17;
}
