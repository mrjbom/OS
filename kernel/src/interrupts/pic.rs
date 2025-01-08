/// Master and slave Programmable Interrupt Controllers
pub static mut PICS: pic8259::ChainedPics = unsafe { pic8259::ChainedPics::new(32, 32 + 8) };

/// Inits PIC and disable PIC interrupts
///
/// IO APIC must be used, we don't use PIC
pub fn init_and_disable() {
    x86_64::instructions::interrupts::disable();
    #[allow(static_mut_refs)]
    unsafe {
        // Mask all interrupts
        PICS.disable();
        // Init (all lines masked)
        //PICS.initialize();
    };
}
