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

const CPU_EXCEPTIONS_IDT_VECTORS_RANGE: RangeInclusive<u8> = 0..=31;
const PIC_IDT_VECTORS_RANGE: RangeInclusive<u8> = 32..=47;
const LOCAL_APIC_TIMER_IDT_VECTOR: u8 = 48;
const LOCAL_APIC_LINT0_IDT_VECTOR: u8 = 49;
const LOCAL_APIC_LINT1_IDT_VECTOR: u8 = 50;
const LOCAL_APIC_ERROR_IDT_VECTOR: u8 = 51;
const LOCAL_APIC_ISA_IRQ_VECTORS_RANGE: RangeInclusive<u8> = 52..=67;
const LOCAL_APIC_SPURIOUS_IDT_VECTOR: u8 = 255;

/// A general handler function for an interrupt or an exception with the interrupt/exception index and an optional error code
///
/// 0-31    CPU exceptions<br>
/// 32-47   PIC hardware interrupts (used before APIC initialization)<br>
/// 48      Local APIC Timer<br>
/// 49      Local APIC LINT0<br>
/// 50      Local APIC LINT1<br>
/// 51      Local APIC Error<br>
/// 52-67   ISA IRQ's from PIC hardware interrupts remaped after APIC initialization
/// 255     Local APIC Spurious-Interrupt (handler must do nothing (and even don't send an EOI))
pub fn general_interrupt_handler(
    interrupt_stack_frame: InterruptStackFrame,
    index: u8,
    error_code: Option<u64>,
) {
    match index {
        index if CPU_EXCEPTIONS_IDT_VECTORS_RANGE.contains(&index) => {
            // CPU Exception
            let exception =
                ExceptionVector::try_from(index).expect("Invalid exception vector number");

            match exception {
                ExceptionVector::Page => {
                    let cr2_virtual_address =
                        x86_64::registers::control::Cr2::read().expect("Invalid address in CR2");
                    panic!(
                        "Exception: {exception:?}\n\
                        Error code: {error_code:#?}\n\
                        CR2: 0x{cr2_virtual_address:X}
                        {interrupt_stack_frame:#?}"
                    );
                }
                _ => {
                    panic!(
                        "Exception: {exception:?}\n\
                        Error code: {error_code:#?}\n\
                        {interrupt_stack_frame:#?}"
                    );
                }
            }
        }
        index if PIC_IDT_VECTORS_RANGE.contains(&index) => {
            // PIT interrupt
            if index == 32 {
                pit::handler();
            }

            crate::serial_println_lock_free!("PIC IRQ interrupt {index}");

            #[allow(static_mut_refs)]
            unsafe {
                pic::PICS.notify_end_of_interrupt(index);
            };
        }
        LOCAL_APIC_TIMER_IDT_VECTOR => {
            crate::serial_println_lock_free!("LOCAL APIC TIMER interrupt");
            apic::send_eoi();
        }
        LOCAL_APIC_LINT0_IDT_VECTOR => {
            crate::serial_println_lock_free!("LOCAL APIC LINT0 interrupt");
            apic::send_eoi();
        }
        LOCAL_APIC_LINT1_IDT_VECTOR => {
            crate::serial_println_lock_free!("LOCAL APIC LINT1 interrupt");
            apic::send_eoi();
        }
        LOCAL_APIC_ERROR_IDT_VECTOR => {
            crate::serial_println_lock_free!("LOCAL APIC ERROR interrupt");
            apic::send_eoi();
        }
        LOCAL_APIC_SPURIOUS_IDT_VECTOR => {
            crate::serial_println_lock_free!("LOCAL APIC SPURIOUS interrupt");
        }
        index if LOCAL_APIC_ISA_IRQ_VECTORS_RANGE.contains(&index) => {
            crate::serial_println_lock_free!("IO APIC ISA IRQ interrupt {index}");
            apic::send_eoi()
        }
        _ => {
            unreachable!("Unexpected interrupt number: {index}!");
        }
    }
}
