/// Programmable Interval Timer
///
/// Only used to calibrate other timers if HPET is not available, since I'm too lazy to deal with this ancient shit.
// http://www.brokenthorn.com/Resources/OSDev16.html
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

const BASE_FREQ: u32 = 1193182;

const OCW_MASK_MODE: u8 = 0xE; // 00001110
const OCW_MASK_RL: u8 = 0x30; // 00110000
const OCW_RL_DATA: u8 = 0x30; // 110000
const OCW_MASK_COUNTER: u8 = 0xC0; // 11000000
const OCW_COUNTER_0: u8 = 0x0; // 00000000
const OCW_MODE_SQUAREWAVEGEN: u8 = 0x6; // 0110
const REG_COMMAND: u16 = 0x43;
const REG_COUNTER0: u16 = 0x40;

// Synchronization:
// I believe atomic access will ensure valid counter operation, the LOCK prefix when writing will prevent other cores from using this variable.
// I assume that ONLY ONE core will increase the counter, because it is the only one that will handle the RTC interrupt and several cores will not be able to try to increment the counter.
static TICK_COUNTER: AtomicU64 = AtomicU64::new(0);
static MILLISECONDS_PER_TICK: AtomicU32 = AtomicU32::new(0);

/// Inits and starts PIT interrupts
///
/// Interval of interrupt in ms (1-54)
pub fn init(interval_in_milliseconds: u32) {
    // Max freq 1193182, with divisor 1, interval 0.00083 ms (1 without float pointer ops)
    // Min freq 18.2, with divisor 65535, interval 54.94 ms (54 without float pointer ops)
    assert!(
        (1..=54).contains(&interval_in_milliseconds),
        "Invalid PIC interval"
    );

    let freq: u32 = 1000 / interval_in_milliseconds;

    assert!(
        BASE_FREQ / freq >= 1 && BASE_FREQ / freq <= 65535,
        "Invalid PIT frequency calculated, bug"
    );
    let divisor: u16 = (BASE_FREQ / freq) as u16;
    // (godbolt tested) With Acquire just mov is used, with SeqCst xchg used, thats blocks the bus (like lock prefix).
    MILLISECONDS_PER_TICK.store(interval_in_milliseconds, Ordering::SeqCst);

    // Send operational command
    let mut ocw: u8 = 0;
    ocw = (ocw & !OCW_MASK_MODE) | OCW_MODE_SQUAREWAVEGEN;
    ocw = (ocw & !OCW_MASK_RL) | OCW_RL_DATA;
    ocw = (ocw & !OCW_MASK_COUNTER) | OCW_COUNTER_0;
    unsafe {
        x86_64::instructions::port::Port::new(REG_COMMAND).write(ocw);
    }

    // Set divisor rate
    unsafe {
        x86_64::instructions::port::Port::<u8>::new(REG_COUNTER0).write((divisor & 0xFF) as u8);
        x86_64::instructions::port::Port::<u8>::new(REG_COUNTER0)
            .write(((divisor >> 8) & 0xFF) as u8);
    }
}

#[inline]
pub fn tick_interrupt_handler() {
    // I checked in godbolt and lock prefix is generated.
    TICK_COUNTER.fetch_add(1, Ordering::AcqRel);
}

#[inline]
pub fn get_ticks_counter() -> u64 {
    // godbolt tested
    // The lock prefix is not generated at any Ordering values (even if SeqCst is used), but it is not required, because we just read the data, everything is safe.
    // Ordering is only required for the compiler, I think Acquire will suffice as there is nothing to reorder.
    TICK_COUNTER.load(Ordering::Acquire)
}

/// Sleeps
pub fn sleep(milliseconds: u32) {
    let start_tick = TICK_COUNTER.load(Ordering::Acquire);
    let milliseconds_per_tick = MILLISECONDS_PER_TICK.load(Ordering::Acquire);
    let end_tick = start_tick + ((milliseconds / milliseconds_per_tick) as u64);
    while get_ticks_counter() < end_tick {
        core::hint::spin_loop();
    }
}
