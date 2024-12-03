fn main() {
    let mut args = std::env::args();
    if args.len() < 3 {
        panic!("Wrong arguments number! Need 2: kernel file path and bootable img file path");
    }
    // Skip program name
    args.next();

    let kernel_file_path = args.next().unwrap();
    let kernel_file_path = std::path::Path::new(&kernel_file_path);
    let bootable_img_file_path = args.next().unwrap();
    let bootable_img_file_path = std::path::Path::new(&bootable_img_file_path);

    if !std::path::Path::new(&kernel_file_path).exists() {
        panic!("Failed to find kernel file");
    }

    // Create bootable img
    // Boot config
    let boot_config = bootloader::BootConfig::default();

    let mut bootable_img = bootloader::BiosBoot::new(kernel_file_path);
    bootable_img.set_boot_config(&boot_config);
    let result = bootable_img.create_disk_image(bootable_img_file_path);
    if let Err(error) = result {
        panic!("Failed to create bootable img: {error}");
    }
    println!("Bootable img created: {bootable_img_file_path:?}");
}
