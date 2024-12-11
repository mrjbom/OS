use raw_cpuid::CpuId;
use crate::interrupts::apic::{fill_lvt_timer_register, LocalApicVersion};

/// Inits Local APIC Timer
///
/// APIC interrupts must be enabled
pub fn init() {
    // https://wiki.osdev.org/APIC_Timer#Enabling_APIC_Timer
    // "Set the local APIC timer's divide configuration register"
    if *super::LOCAL_APIC_VERSION.get().unwrap() == LocalApicVersion::Integrated {
        set_divide_configuration_register(16); // Value from wiki "Bochs seems not to handle divide value of 1 properly, so I will use 16."
    }

    // "Configure the local APIC timer's interrupt vector and unmask the timer's IRQ"
    // "Configure the local APIC timer's interrupt vector"
    // The timer is started by writing to the Initial Count Register
    fill_lvt_timer_register();
    // "unmask the timer's IRQ"
    //unsafe { crate::interrupts::pic::PICS.write_masks(0b11111110, 0xFF) };

    // "Set the local APIC timer's initial count"
    log::debug!("APIC unmasked, start");
    set_initial_count_register(9999999);

    x86_64::instructions::interrupts::enable();
    loop {

    }

    // https://wiki.osdev.org/APIC_Timer#Initializing
}

/// Sets frequency divider in Divide Configuration Register
///
/// ## The Descrete APIC uses the bus frequency and is not affected by the local APIC frequency divider.
///
/// Valid values is 1, 2, 4, 8, 16, 32, 64, 128
///
/// Wiki says "Bochs seems not to handle divide value of 1 properly"
fn set_divide_configuration_register(frequency_divider: u16) {
    if *super::LOCAL_APIC_VERSION.get().unwrap() == LocalApicVersion::Descrete {
        // The discrete APIC uses the bus frequency, and it does not make sense to set a local APIC frequency divider.
        panic!("Trying to set frequency divider for Descrete APIC");
    }
    if !frequency_divider.is_power_of_two() || frequency_divider < 1 || frequency_divider > 128 {
        panic!("Trying to set invalid frequency divider");
    }
    if frequency_divider == 1 {
        log::warn!("For APIC Timer wiki says \"Bochs seems not to handle divide value of 1 properly\"");
    }

    // Devide Configuration Register value
    let register_value = match frequency_divider {
        1 => 0b1011,
        2 => 0b0000,
        4 => 0b0001,
        8 => 0b0010,
        16 => 0b0011,
        32 => 0b1000,
        64 => 0b1001,
        128 => 0b1010,
        _ => unreachable!(),
    };

    unsafe {
        super::DIVIDE_CONFIGURATION_REGISTER.write_volatile(register_value);
    }
}
/// Sets Initial Count register
///
/// Write of 0 to the initial-count register effectively stops the local APIC timer, in both one-shot and periodic mode.
pub fn set_initial_count_register(initial_count: u32) {
    unsafe {
        super::INITIAL_COUNT_REGISTER.write_volatile(initial_count);
    }
}
