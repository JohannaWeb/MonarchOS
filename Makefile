.PHONY: build run clean test

ARCH ?= x86_64
LIMINE_VERSION ?= v7.8.1

build:
	cargo build --release -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem

run: build
	@echo "Running in QEMU (requires QEMU to be installed)"
	qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/kernel \
		-m 512M \
		-serial stdio \
		-display none

run-gui: build
	qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/kernel \
		-m 512M \
		-serial stdio

clean:
	cargo clean
	rm -rf target/

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
