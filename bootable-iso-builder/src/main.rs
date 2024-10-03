fn main() {
    let mut args = std::env::args();
    if args.len() < 3 {
        panic!("Wrong arguments number! Need 2: kernel file path and bootable iso file path");
    }
    // Skip program name
    args.next();

    let kernel_file_path = args.next().unwrap();
    let kernel_file_path = std::path::Path::new(&kernel_file_path);
    let bootable_iso_file_path = args.next().unwrap();
    let bootable_iso_file_path = std::path::Path::new(&bootable_iso_file_path);

    if !std::path::Path::new(&kernel_file_path).exists() {
        panic!("Failed to find kernel file");
    }

    // Create bootable iso
    // Boot config
    let boot_config = bootloader::BootConfig::default();

    let mut bootable_iso = bootloader::BiosBoot::new(kernel_file_path);
    bootable_iso.set_boot_config(&boot_config);
    let result = bootable_iso.create_disk_image(bootable_iso_file_path.into());
    if let Err(error) = result {
        panic!("Failed to create bootable iso: {error}");
    }
    println!("Bootable iso created: {bootable_iso_file_path:?}");
}
