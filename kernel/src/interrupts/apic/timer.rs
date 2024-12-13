use crate::interrupts::apic::{fill_lvt_timer_register, LocalApicVersion};
use crate::interrupts::pit;

/// Inits Local APIC Timer, uses PIT
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
    unsafe {
        // IRQ0 unmasked, others IRQ's masked
        #[allow(static_mut_refs)]
        crate::interrupts::pic::PICS.write_masks(0b11111110, 0b11111111)
    };

    // Determine APIC Timer frequency without divider
    x86_64::instructions::interrupts::enable();
    // Frequency without divider (= 1) in Hz (for 1 second)
    // PIT used
    let timer_true_frequency = determine_timer_true_frequency(16);
    log::debug!("Timer freq: {timer_true_frequency}");
    // Disable PIC by masking all interrupts
    x86_64::instructions::interrupts::disable();
    unsafe {
        #[allow(static_mut_refs)]
        crate::interrupts::pic::PICS.disable();
    }

    assert_ne!(
        timer_true_frequency, 0,
        "Failed to detect APIC frequency, bug"
    );

    // https://wiki.osdev.org/APIC_Timer#Initializing
}

/// Sets frequency divider in Divide Configuration Register
///
/// Valid values is 1, 2, 4, 8, 16, 32, 64, 128
///
/// Wiki says "Bochs seems not to handle divide value of 1 properly"
fn set_divide_configuration_register(frequency_divider: u8) {
    if !frequency_divider.is_power_of_two() || frequency_divider < 1 || frequency_divider > 128 {
        panic!("Trying to set invalid frequency divider");
    }
    if frequency_divider == 1 {
        panic!("For APIC Timer wiki says \"Bochs seems not to handle divide value of 1 properly\"");
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
/// Sets Initial Count register and starts or stops timer
///
/// Write non 0 - starts
/// Write 0 - stops
pub fn set_initial_count_register(initial_count: u32) {
    unsafe {
        super::INITIAL_COUNT_REGISTER.write_volatile(initial_count);
    }
}

pub fn get_current_count_register_value() -> u32 {
    unsafe { *super::CURRENT_COUNT_REGISTER }
}

/// Determines the frequency of the Local APIC Timer using PIT sleep<br>
/// Calculates the true frequency as if there is no divisor (1)
///
/// current_frequency_divider may be any, the frequency will be recalculated taking it into account
// todo: Add determining using cpuid: it's a bit more complicated, but more accurate.
fn determine_timer_true_frequency(current_frequency_divider: u8) -> u64 {
    let mut ticks_measures = [0u64; 10];
    for v in ticks_measures.iter_mut() {
        // Sleep betwen measures
        pit::sleep(5);

        let initial_count = u32::MAX;

        // Start APIC Timer
        set_initial_count_register(initial_count);

        // Sleep 10 ms
        pit::sleep(10);

        // Read current count register
        let current_count = get_current_count_register_value();

        // Ticks in 10 ms
        *v = (initial_count - current_count) as u64;
    }

    // Disable APIC Timer
    set_initial_count_register(0);

    // Ticks in 10 ms
    let average_ticks: u64 = ticks_measures.iter().sum::<u64>() / ticks_measures.len() as u64;

    // Ticks in 1000 ms without divisor = true HZ
    average_ticks * current_frequency_divider as u64 * 100
}
