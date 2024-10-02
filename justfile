KERNEL_DEBUG_FILE_PATH := "target/x86_64-unknown-none/debug/kernel" # kernel elf file
BOOTABLE_ISO_FILE_PATH := "bootable.iso"

list:
	@just --list

build:
	@echo "Building..."
	@echo "Building kernel"
	cargo build --package kernel --config kernel/config.toml
	@echo "Creating bootable iso"
	cargo run --package bootable-iso-builder -- {{KERNEL_DEBUG_FILE_PATH}} {{BOOTABLE_ISO_FILE_PATH}}

run: build
	qemu-system-x86_64 {{BOOTABLE_ISO_FILE_PATH}}
