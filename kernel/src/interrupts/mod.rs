use x86_64::structures::idt::InterruptStackFrame;

mod idt;

/// Master and slave Programmable Interrupt Controllers
static mut PICS: pic8259::ChainedPics = unsafe { pic8259::ChainedPics::new(32, 32 + 8) };

/// Inits and enable interrupts
pub fn init() {
    // Init IDT
    idt::init();

    // Remap PIC
    #[allow(static_mut_refs)]
    unsafe {
        PICS.initialize()
    };

    // Enable interrupts
    x86_64::instructions::interrupts::enable();
}

/// A general handler function for an interrupt or an exception with the interrupt/exception index and an optional error code
pub fn general_handler_func(
    _interrupt_stack_frame: InterruptStackFrame,
    index: u8,
    _error_code: Option<u64>,
) {
    if index < 32 {
        // Exception
    } else {
        // Hardware PIC interrupt
        #[allow(static_mut_refs)]
        unsafe {
            PICS.notify_end_of_interrupt(index)
        };
    }
}
