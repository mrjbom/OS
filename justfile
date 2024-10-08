KERNEL_DEBUG_FILE_PATH := "target/x86_64-unknown-none/debug/kernel" # kernel elf file
BOOTABLE_ISO_FILE_PATH := "bootable.iso"

#RUN_DEV_QEMU_FLAGS := "-serial file:serial.log -monitor stdio"

# QEMU monitor and stdio in terminal
# Press Ctrl-A then C to switch between serial and monitor
RUN_DEV_QEMU_FLAGS := "-serial mon:stdio -display none"

list:
	@just --list

build-dev:
	@echo "Building..."
	@echo "Building kernel"
	cargo build --package kernel --config kernel/config.toml
	@echo "Creating bootable iso"
	cargo run --package bootable-iso-builder -- {{KERNEL_DEBUG_FILE_PATH}} {{BOOTABLE_ISO_FILE_PATH}}

run-dev: build-dev
	qemu-system-x86_64 {{BOOTABLE_ISO_FILE_PATH}} {{RUN_DEV_QEMU_FLAGS}}
