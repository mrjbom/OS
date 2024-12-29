use x86_64::instructions::segmentation::Segment;
use x86_64::registers::segmentation::SegmentSelector;
/// Inits GDT
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::PrivilegeLevel;

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();

/// Creates and loads GDT
#[allow(static_mut_refs)]
pub fn init() {
    unsafe {
        // Null Descriptor already in GDT
        // GDT[1] Kernel Code
        GDT.append(Descriptor::kernel_code_segment());
        // GDT[2] Kernel Data
        GDT.append(Descriptor::kernel_data_segment());
        // GDT[3] User Code
        GDT.append(Descriptor::user_code_segment());
        // GDT[4] User Data
        GDT.append(Descriptor::user_data_segment());

        // lgdt
        GDT.load();

        // Set segment registers
        // CS, DS, SS, ES
        // FS and GS not used
        x86_64::instructions::segmentation::CS::set_reg(SegmentSelector::new(
            1,
            PrivilegeLevel::Ring0,
        ));
        x86_64::instructions::segmentation::DS::set_reg(SegmentSelector::new(
            2,
            PrivilegeLevel::Ring0,
        ));
        x86_64::instructions::segmentation::SS::set_reg(SegmentSelector::new(
            2,
            PrivilegeLevel::Ring0,
        ));
        x86_64::instructions::segmentation::ES::set_reg(SegmentSelector::new(
            2,
            PrivilegeLevel::Ring0,
        ));
    }
}
