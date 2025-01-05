use core::ops::RangeInclusive;
use x86_64::structures::idt::{ExceptionVector, InterruptStackFrame};

pub mod apic;
pub mod idt;
pub mod pic;
pub mod pit;

/// Fills IDT, inits IO APIC and bootstrap processor's Local APIC, enables interrupts
pub fn init() {
    x86_64::instructions::interrupts::disable();

    // Init and disable PIC
    pic::init_and_disable();

    // Init Local APIC and IO APIC
    apic::init();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();
}
