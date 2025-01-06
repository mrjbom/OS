use acpi_lib::{AcpiError, AcpiResult};
use acpi_lib::hpet::HpetTable;
use spin::Once;
use crate::acpi::ACPI_TABLES;

pub mod pit;

enum TimerName {
    PIT,
    HPET,
    ITSC,
    LapicTimer,
}

static HPET_SUPPORTED: Once<bool> = Once::new();

static ITSC_SUPPORTED: Once<bool> = Once::new();

// All timers count ticks
// All timers can have one-shot and periodic modes
// All timers can measure the time between two points in time
// All timers can allow sleep()

// Which timers and what I want to use them for:
// 1. PIT - Only for calibrating other timers (ITSC and Local APIC Timer) if HPET is not available.
// 2. HPET - To calibrate ITSC and Local APIC Timer, as a system-wide timer
// (to time and measure time or generate interrupts in one-shot mode) if ITSC is not available.
// 3. ITSC - As a system-wide timer to time and measure time.
// 4. Local APIC Timer - To generate scheduler interrupts for each core.

/// Inits PIT, HPET, Invariant TSC and bootstrap processor's Local APIC Timer
pub fn init() {
    x86_64::instructions::interrupts::disable();
    // PIT is only used in the role of calibration timer if HPET is not available
    pit::init(1);

    // Check timers support
    // Check HPET support in ACPI tables
    let hpet_table_result = ACPI_TABLES.get().unwrap().lock().find_table::<HpetTable>();
    match hpet_table_result {
        Ok(hpet_table_physical_mapping) => {
            HPET_SUPPORTED.call_once(|| true);
            log::info!("HPET supported");
        }
        Err(ref err) => {
            // If table not found - HPET not supported
            if !matches!(hpet_table_result, Err(AcpiError::TableMissing(_))) {
                HPET_SUPPORTED.call_once(|| false);
                log::info!("HPET not supported");
            } else {
                // Some ACPI error occurs
                panic!("Failed to get HPET table in ACPI: {err:?}");
            }
        }
    }

    // Check Invariant TSC support using cpuid (works on Intel and AMD)
    let cpuid = raw_cpuid::CpuId::new();
    let has_invariant_tsc = cpuid.get_advanced_power_mgmt_info().expect("Failed to get cpuid advanced power management info").has_invariant_tsc();
    ITSC_SUPPORTED.call_once(|| has_invariant_tsc);
    match has_invariant_tsc {
        true => {
            log::info!("Invariant TSC supported");
        }
        false => {
            log::info!("Invariant TSC not supported");
        }
    }
}
