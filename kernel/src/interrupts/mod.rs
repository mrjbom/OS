use core::ops::RangeInclusive;
use x86_64::structures::idt::{ExceptionVector, InterruptStackFrame};

pub mod apic;
pub mod idt;
pub mod pic;

/// Inits IDT, PIC and enable interrupts
///
/// APIC is not used. Switching to APIC is done using the go_to_apic() function.
pub fn init() {
    // Fill IDT
    idt::init();

    // Remap and init PIC
    pic::init();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();
}

/// Disables PIC and inits local APIC, enables interrupts
pub fn go_to_apic() {
    // Init local APIC
    apic::init();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();
}

const CPU_EXCEPTIONS_IDT_VECTORS_RANGE: RangeInclusive<u8> = 0..=31;
const PIC_IDT_VECTORS_RANGE: RangeInclusive<u8> = 32..=47;
const ACPI_TIMER_IDT_VECTOR: u8 = 48;
const ACPI_LINT0_IDT_VECTOR: u8 = 49;
const ACPI_LINT1_IDT_VECTOR: u8 = 50;
const ACPI_ERROR_IDT_VECTOR: u8 = 51;
const ACPI_SPURIOUS_IDT_VECTOR: u8 = 255;

/// A general handler function for an interrupt or an exception with the interrupt/exception index and an optional error code
///
/// 0-31    CPU exceptions<br>
/// 32-47   PIC hardware interrupts (used before APIC initialization)<br>
/// 48      ACPI Timer<br>
/// 49      ACPI LINT0<br>
/// 50      ACPI LINT1<br>
/// 51      ACPI Error<br>
/// 255     ACPI Spurious-Interrupt Vector Register (handler must do nothing (and even don't send an EOI))
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
            // PIC interrupt
            #[allow(static_mut_refs)]
            unsafe {
                pic::PICS.notify_end_of_interrupt(index)
            };
        }
        ACPI_TIMER_IDT_VECTOR => {
            crate::serial_println!("ACPI TIMER interrupt");
        }
        ACPI_LINT0_IDT_VECTOR => {
            crate::serial_println!("ACPI LINT0 interrupt");
        }
        ACPI_LINT1_IDT_VECTOR => {
            crate::serial_println!("ACPI LINT1 interrupt");
        }
        ACPI_ERROR_IDT_VECTOR => {
            crate::serial_println!("ACPI ERROR interrupt");
        }
        ACPI_SPURIOUS_IDT_VECTOR => {
            crate::serial_println!("ACPI SPURIOUS interrupt");
        }
        _ => {
            unreachable!("Unexpected interrupt number!");
        }
    }
}
