use core::ops::RangeInclusive;
use x86_64::structures::idt::{ExceptionVector, InterruptStackFrame};

pub mod apic;
pub mod idt;
pub mod pic;
pub mod pit;

/// Inits IDT, PIC, PIT and enable interrupts
///
/// APIC is not used. Switching to APIC is done using the go_to_apic() function.
pub fn init() {
    // Fill IDT
    idt::init();

    // Remap and init PIC
    pic::init();

    // Init and start PIT
    pit::init(1);

    // Enable interrupts
    x86_64::instructions::interrupts::enable();
}

/// Disables PIC and inits local APIC, IO APIC, enables interrupts
pub fn go_to_apic() {
    x86_64::instructions::interrupts::disable();

    // Init Local APIC and IO APIC
    apic::init();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();
}
