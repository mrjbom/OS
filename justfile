KERNEL_DEBUG_FILE_PATH := "target/x86_64-unknown-none/debug/kernel" # kernel elf file
BOOTABLE_ISO_FILE_PATH := "bootable.iso"

#RUN_DEV_QEMU_FLAGS := "-serial file:serial.log -monitor stdio"

# QEMU monitor and stdio in terminal
# Press Ctrl-A then C to switch between serial and monitor
RUN_DEV_QEMU_FLAGS := "-serial mon:stdio -display none"

list:
	@just --list

# Build debug version
build-dev:
	@echo "Building..."
	@echo "Building kernel"
	cargo build --package kernel --config kernel/config.toml
	@echo "Creating bootable iso"
	cargo run --package bootable-iso-builder -- {{KERNEL_DEBUG_FILE_PATH}} {{BOOTABLE_ISO_FILE_PATH}}

# build-dev with verbose flags
build-dev-verbose:
	@echo "Building..."
	@echo "Building kernel"
	cargo build --package kernel --config kernel/config.toml --verbose
	@echo "Creating bootable iso"
	cargo run --package bootable-iso-builder -- {{KERNEL_DEBUG_FILE_PATH}} {{BOOTABLE_ISO_FILE_PATH}}

# Build and run debug version
run-dev: build-dev
	qemu-system-x86_64 {{BOOTABLE_ISO_FILE_PATH}} {{RUN_DEV_QEMU_FLAGS}}

# Alias for build-dev
b: build-dev

# Alias for run-dev
r: run-dev

# Runs qemu for gdb debug
rdbg: build-dev
	qemu-system-x86_64 {{BOOTABLE_ISO_FILE_PATH}} {{RUN_DEV_QEMU_FLAGS}} -s -S
