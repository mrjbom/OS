pub mod apic;
pub mod idt;
pub mod pic;

/// Fills IDT, inits IO APIC and bootstrap processor's Local APIC, but it doesn't enable interrupts
pub fn init() {
    x86_64::instructions::interrupts::disable();

    // Init and disable PIC
    pic::init_and_disable();

    // Init Local APIC and IO APIC
    apic::init();
}
