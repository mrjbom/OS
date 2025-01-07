use crate::acpi::ACPI_TABLES;
use crate::memory_management::virtual_memory_manager;
use acpi_lib::{AcpiError, AcpiTable, HpetInfo};
use bitfield::bitfield;
use core::time::Duration;
use fixed::types::extra::U12;
use fixed::FixedU64;
use spin::Once;
use x86_64::{PhysAddr, VirtAddr};

static HPET_TIMER: Once<Option<HPETTimer>> = Once::new();

/// Detects and creates HPET (but not starts, only detects)
pub fn init() {
    // Have HPET?
    let hpet_info = HpetInfo::new(&ACPI_TABLES.get().unwrap().lock());
    if let Err(ref err) = hpet_info {
        // If table not found - HPET not supported
        if matches!(hpet_info, Err(AcpiError::TableMissing(_))) {
            log::info!("HPET not supported");
            HPET_TIMER.call_once(|| None);
            return;
        } else {
            // Some ACPI error occurs
            panic!("Failed to get HPET info from ACPI tables: {err:?}");
        }
    }

    // HPET in System Memory?
    // It is unlikely to encounter System I/O, I assume System Memory used
    // In this version, the library panics when creating HpetInfo::new() if HPET uses System I/O, but I'll check it out anyway.
    unsafe {
        let hpet_table_ptr = ACPI_TABLES
            .get()
            .unwrap()
            .lock()
            .find_table::<acpi_lib::hpet::HpetTable>()
            .unwrap()
            .virtual_start()
            .as_ptr();
        (*hpet_table_ptr)
            .validate()
            .expect("Invalid HPET table detected");
        // Since the library is written by strange people, the fields of the HpetTable structure are private, let's check it manually using a pointer.
        // TODO: Contribute with public fields in HpetTable
        // BASE_ADDRESS is 12 byte at 40 byte offset
        // If first byte is 0 - System Memory
        // If first byte is 1 - System IO
        let base_address_first_byte = *(hpet_table_ptr.byte_add(40) as *mut u8);
        assert_eq!(base_address_first_byte, 0, "HPET uses System IO");
    }

    // HPET detected
    log::info!("HPET supported");
    let hpet_info = hpet_info.unwrap();

    // Create HPET control object
    HPET_TIMER.call_once(|| Some(HPETTimer::new(hpet_info)));

    // Run main counter and interrupts (if comparators has enabled interrupts)
    run();
}

#[inline]
pub fn is_supported() -> bool {
    HPET_TIMER.get().unwrap().is_some()
}

// HPET control structure
struct HPETTimer {
    hpet_acpi_info: HpetInfo,
    base_address: VirtAddr,
    /// Period in femtoseconds (femtoseconds per tick)
    period_in_femtoseconds: FixedU64<U12>,
    /// Period in nanoseconds (nanoseconds per tick)
    period_in_nanoseconds: FixedU64<U12>,
    frequency: FixedU64<U12>,
}

impl HPETTimer {
    /// Creates HPET timer, checks cap's
    fn new(hpet_acpi_info: HpetInfo) -> Self {
        // Get base address
        let base_address = virtual_memory_manager::virt_addr_in_cpmm_from_phys_addr(PhysAddr::new(
            hpet_acpi_info.base_address as u64,
        ));

        // Check period
        let general_capabilities_and_id_register_value =
            Self::read_general_capabilities_and_id_register_value(base_address);
        let counter_clock_period: u64 =
            general_capabilities_and_id_register_value.counter_clock_period();
        // Period <= 100 nanoseconds
        assert!(
            counter_clock_period <= 0x05F5E100,
            "HPET has incorrect counter clock period"
        );

        // The 32-bit counter will overflow after about 7 minutes (for 10 MHz)
        // A 32-bit counter is unlikely to be encountered because the specification recommends having a 64-bit
        assert_eq!(
            general_capabilities_and_id_register_value.count_size_cap(),
            true,
            "HPET don't have 64-bit main counter"
        );

        // Must have minimum 3 comparators
        assert!(
            general_capabilities_and_id_register_value.number_timers_cap() + 1 >= 3,
            "Incorrect number of comparators in HPET"
        );

        // Calculate frequency (Min: 10 MHz) using counter clock period (in femtoseconds)
        // f = 10^15 / period
        // FixedU64<U12> has delta 1/2^12 = 0.00024414062
        let femtoseconds_in_second: FixedU64<U12> =
            FixedU64::<U12>::from_num(1_000_000_000_000_000u64);
        let period_in_femtoseconds: FixedU64<U12> = FixedU64::<U12>::from_num(counter_clock_period);
        let frequency: FixedU64<U12> = femtoseconds_in_second / counter_clock_period;

        // For Duration calculation
        let period_in_nanoseconds: FixedU64<U12> = period_in_femtoseconds / 1_000_000;
        assert!(
            period_in_nanoseconds > FixedU64::<U12>::DELTA,
            "Calculated period in nanoseconds small than delta"
        );

        Self {
            hpet_acpi_info,
            base_address,
            period_in_femtoseconds,
            period_in_nanoseconds,
            frequency,
        }
    }
    /// General Capabilities And ID Register
    #[inline]
    fn read_general_capabilities_and_id_register_value(
        base_address: VirtAddr,
    ) -> GeneralCapabilitiesAndIdRegisterValue {
        // Offset: 0x000 - 0x007 (8 bytes)
        let register_value: u64 = unsafe { *(base_address.as_ptr()) };
        GeneralCapabilitiesAndIdRegisterValue(register_value)
    }

    /// General Configuration Register
    #[inline]
    fn read_general_configuration_register_value(&self) -> GeneralConfigurationRegisterValue {
        // Offset: 0x010 - 0x017 (8 bytes)
        let register_value: u64 = unsafe { *(self.base_address.as_ptr::<u64>().byte_add(0x010)) };
        GeneralConfigurationRegisterValue(register_value)
    }

    /// General Configuration Register
    #[inline]
    fn write_general_configuration_register_value(
        &self,
        register_value: GeneralConfigurationRegisterValue,
    ) {
        // Offset: 0x010 - 0x017 (8 bytes)
        unsafe {
            let register_ptr = self
                .base_address
                .as_mut_ptr::<GeneralConfigurationRegisterValue>()
                .byte_add(0x010);
            register_ptr.write_volatile(register_value);
        }
    }

    /// Main Counter Value Register
    #[inline]
    fn read_main_counter_value_register(&self) -> u64 {
        // 0x0F0 - 0x0F7 (8 bytes)
        unsafe {
            let register_value: u64 =
                unsafe { *(self.base_address.as_ptr::<u64>().byte_add(0x0F0)) };
            register_value
        }
    }
}

/// Runs main counter and timer interrupts are allowed if enabled
///
/// See General Configuration Register::ENABLE_CNF = 1
pub fn run() {
    let hpet_timer = HPET_TIMER.get().unwrap().as_ref().unwrap();
    let mut register_value = hpet_timer.read_general_configuration_register_value();
    register_value.set_enable_cnf(true);
    hpet_timer.write_general_configuration_register_value(register_value);
}

/// Halts main counter and disables interrupts
///
/// See General Configuration Register::ENABLE_CNF = 0
pub fn halt() {
    let hpet_timer = HPET_TIMER.get().unwrap().as_ref().unwrap();
    let mut register_value = hpet_timer.read_general_configuration_register_value();
    register_value.set_enable_cnf(false);
    hpet_timer.write_general_configuration_register_value(register_value);
}

#[inline]
pub fn get_current_ticks() -> u64 {
    let hpet_timer = HPET_TIMER.get().unwrap().as_ref().unwrap();
    hpet_timer.read_main_counter_value_register()
}

#[inline]
pub fn get_current_ticks_as_duration() -> Duration {
    let current_ticks = get_current_ticks();
    ticks_to_duration(current_ticks)
}

#[inline]
pub fn ticks_to_duration(ticks: u64) -> Duration {
    // 1 tick = n nanoseconds
    let nanoseconds_per_tick = HPET_TIMER
        .get()
        .unwrap()
        .as_ref()
        .unwrap()
        .period_in_nanoseconds;
    Duration::from_nanos((ticks * nanoseconds_per_tick).to_num())
}

#[inline]
pub fn duration_to_ticks(duration: Duration) -> u64 {
    // 1 tick = n nanoseconds
    let nanoseconds_per_tick = HPET_TIMER
        .get()
        .unwrap()
        .as_ref()
        .unwrap()
        .period_in_nanoseconds;
    let nanoseconds = FixedU64::<U12>::from_num(duration.as_nanos());
    (nanoseconds / nanoseconds_per_tick).to_num()
}

pub fn sleep(sleep_dutation: Duration) {
    let hpet_timer = HPET_TIMER.get().unwrap().as_ref().unwrap();

    let start_tick_value = hpet_timer.read_main_counter_value_register();
    let wait_ticks = duration_to_ticks(sleep_dutation);
    let end_tick_value = start_tick_value + wait_ticks;

    while hpet_timer.read_main_counter_value_register() < end_tick_value {
        core::hint::spin_loop();
    }
}

bitfield! {
    struct GeneralCapabilitiesAndIdRegisterValue(u64);
    impl Debug;
    counter_clock_period, _: 63, 32;
    vendor_id, _: 31, 16;
    legacy_replacement_cap, _: 15;
    count_size_cap, _: 13;
    number_timers_cap, _: 12, 8;
    revision_id, _: 7, 0;
}

bitfield! {
    struct GeneralConfigurationRegisterValue(u64);
    impl Debug;
    legacy_replacement_cnf, set_legacy_replacement_cnf: 1;
    enable_cnf, set_enable_cnf: 0;
}
