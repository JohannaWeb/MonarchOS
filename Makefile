.PHONY: build run run-gui clean test check clippy fmt fmt-check help limine-bootloader iso

ARCH ?= x86_64
LIMINE_VERSION ?= v7.x-binary
BUILD_DIR ?= .build
DEPS_DIR ?= .deps
ISO_ROOT := $(BUILD_DIR)/iso_root
ISO_IMAGE := $(BUILD_DIR)/kernel.iso
LIMINE_DIR := $(DEPS_DIR)/limine

build:
	cargo build --release -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem

limine-bootloader:
	@mkdir -p $(DEPS_DIR)
	@if [ ! -d "$(LIMINE_DIR)" ]; then \
		git clone https://github.com/limine-bootloader/limine.git --branch $(LIMINE_VERSION) --depth=1 $(LIMINE_DIR); \
		cd $(LIMINE_DIR) && $(MAKE); \
	else \
		echo "Limine bootloader already present"; \
	fi

iso: build limine-bootloader
	@mkdir -p $(ISO_ROOT)/boot/limine
	@mkdir -p $(ISO_ROOT)/EFI/BOOT
	@cp limine.cfg $(ISO_ROOT)/
	@cp $(LIMINE_DIR)/limine-bios.sys $(ISO_ROOT)/boot/limine/
	@cp $(LIMINE_DIR)/limine-bios-cd.bin $(ISO_ROOT)/boot/limine/
	@cp $(LIMINE_DIR)/limine-uefi-cd.bin $(ISO_ROOT)/boot/limine/
	@cp $(BUILD_DIR)/target/x86_64-unknown-none/release/kernel $(ISO_ROOT)/boot/kernel
	@cp $(LIMINE_DIR)/BOOTX64.EFI $(ISO_ROOT)/EFI/BOOT/
	@xorriso -as mkisofs \
		-b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		$(ISO_ROOT) -o $(ISO_IMAGE)
	@$(LIMINE_DIR)/limine bios-install $(ISO_IMAGE)
	@echo "ISO built successfully: $(ISO_IMAGE)"

run: iso
	@echo "Running in QEMU (requires QEMU to be installed)"
	qemu-system-x86_64 -cdrom $(ISO_IMAGE) \
		-m 512M \
		-cpu max \
		-serial stdio \
		-display none

run-gui: iso
	qemu-system-x86_64 -cdrom $(ISO_IMAGE) \
		-m 512M \
		-cpu max \
		-serial stdio

clean:
	cargo clean
	rm -rf target/ $(BUILD_DIR)/ $(DEPS_DIR)/

test:
	cargo test

check:
	cargo check

clippy:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

help:
	@echo "Available targets:"
	@echo "  build       - Build the kernel"
	@echo "  iso         - Build bootable ISO at $(ISO_IMAGE)"
	@echo "  run         - Build and run in QEMU (headless)"
	@echo "  run-gui     - Build and run in QEMU (with display)"
	@echo "  clean       - Clean build artifacts"
	@echo "  test        - Run tests"
	@echo "  check       - Check code without building"
	@echo "  clippy      - Run clippy linter"
	@echo "  fmt         - Format code"
	@echo "  fmt-check   - Check code formatting"
