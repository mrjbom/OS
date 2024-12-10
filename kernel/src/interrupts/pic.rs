/// Master and slave Programmable Interrupt Controllers
pub static mut PICS: pic8259::ChainedPics = unsafe { pic8259::ChainedPics::new(32, 32 + 8) };

/// Remaps and inits PIC
pub fn init() {
    // Remap PIC
    #[allow(static_mut_refs)]
    unsafe {
        PICS.initialize()
    };
}
