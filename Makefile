.PHONY: build run run-gui clean test check clippy fmt fmt-check help limine-bootloader iso

ARCH ?= x86_64
LIMINE_VERSION ?= v7.x-binary

build:
	cargo build --release -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem

limine-bootloader:
	@if [ ! -d "limine" ]; then \
		git clone https://github.com/limine-bootloader/limine.git --branch $(LIMINE_VERSION) --depth=1; \
		cd limine && $(MAKE); \
	else \
		echo "Limine bootloader already present"; \
	fi

iso: build limine-bootloader
	@mkdir -p iso_root/boot/limine
	@mkdir -p iso_root/EFI/BOOT
	@cp limine.cfg iso_root/
	@cp limine/limine-bios.sys iso_root/boot/limine/
	@cp limine/limine-bios-cd.bin iso_root/boot/limine/
	@cp limine/limine-uefi-cd.bin iso_root/boot/limine/
	@cp target/x86_64-unknown-none/release/kernel iso_root/boot/kernel
	@cp limine/BOOTX64.EFI iso_root/EFI/BOOT/
	@xorriso -as mkisofs \
		-b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o kernel.iso
	@./limine/limine bios-install kernel.iso
	@echo "ISO built successfully: kernel.iso"

run: iso
	@echo "Running in QEMU (requires QEMU to be installed)"
	qemu-system-x86_64 -cdrom kernel.iso \
		-m 512M \
		-serial stdio \
		-display none

run-gui: iso
	qemu-system-x86_64 -cdrom kernel.iso \
		-m 512M \
		-serial stdio

clean:
	cargo clean
	rm -rf target/ iso_root/ kernel.iso limine/

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
	@echo "  run         - Build and run in QEMU (headless)"
	@echo "  run-gui     - Build and run in QEMU (with display)"
	@echo "  clean       - Clean build artifacts"
	@echo "  test        - Run tests"
	@echo "  check       - Check code without building"
	@echo "  clippy      - Run clippy linter"
	@echo "  fmt         - Format code"
	@echo "  fmt-check   - Check code formatting"
