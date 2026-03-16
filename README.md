# MonarchOS

> A Unix-like operating system written in Rust for x86-64 architecture

A from-scratch OS kernel with a focus on safety, modularity, and modern systems design. MonarchOS leverages Rust's memory safety guarantees to build a stable, efficient kernel while maintaining a clean architecture suitable for education and production use.

**Status**: Early development - Bootable kernel skeleton with core infrastructure

## Goals

- Full Unix-like system with POSIX compliance
- Type-safe and memory-safe kernel implementation in Rust
- Modular, extensible architecture
- Comprehensive process and memory management
- Support for modern x86-64 hardware via limine bootloader
- Educational value and clean codebase

## Architecture

```
src/
├── arch/           # Architecture-specific code (x86_64)
│   ├── gdt.rs      # Global Descriptor Table
│   ├── idt.rs      # Interrupt Descriptor Table
│   └── paging.rs   # Virtual memory management
├── memory/         # Memory management subsystem
│   ├── allocator.rs# Kernel heap allocator
│   └── manager.rs  # Virtual memory tracking
├── process/        # Process/task management
│   ├── scheduler.rs# Process scheduler
│   └── task.rs     # Task structures
├── filesystem/     # Filesystem subsystem
│   ├── vfs.rs      # Virtual filesystem
│   └── ext2.rs     # ext2 filesystem driver
├── io/             # Input/output subsystems
│   ├── serial.rs   # UART/serial port driver
│   └── console.rs  # Framebuffer console
├── sync/           # Synchronization primitives
│   ├── spinlock.rs # Spinlock implementation
│   └── semaphore.rs# Semaphore implementation
├── main.rs         # Kernel entry point
└── lib.rs          # Core kernel library
```

## Prerequisites

- **Rust**: Nightly toolchain (specified in `rust-toolchain.toml`)
- **QEMU**: For emulation and testing
- **Make**: Build automation
- **Build tools**: gcc/ld (for linking)

Rust's nightly is required for unstable features like `#[lang = "..."]` which are essential for bare-metal kernel development.

## Building

```bash
# Build the kernel
cargo build

# Build release binary (optimized)
cargo build --release
```

### Using Make

```bash
make build       # Compile kernel
make run         # Run in QEMU headless
make run-gui     # Run in QEMU with display
make check       # Check without compiling
make clean       # Clean build artifacts
make test        # Run tests
```

## Project Status

### Implemented
- Bootloader integration via limine protocol
- Kernel entry point and basic initialization
- Memory map detection from bootloader
- Framebuffer output support
- Spinlock and semaphore synchronization primitives
- Basic exception handling stub
- Proper bare-metal configuration (no_std, panic=abort)
- Compiler intrinsics (memcpy, memset, memcmp, memmove)

### In Progress / TODO
- [ ] **GDT Setup**: Configure Global Descriptor Table for privilege levels
- [ ] **IDT Implementation**: Interrupt and exception handling
- [ ] **Paging**: Virtual memory management with page tables
- [ ] **Heap Allocator**: Dynamic memory allocation
- [ ] **Interrupt Handlers**: Handle IRQs and exceptions
- [ ] **Process Scheduler**: Task scheduling and context switching
- [ ] **System Calls**: User/kernel interface
- [ ] **Filesystem**: ext2 filesystem implementation
- [ ] **Device Drivers**: AHCI, keyboard, network
- [ ] **Userspace**: Shell and POSIX utilities
- [ ] **POSIX Compliance**: Full standards compliance

## Development

### Code Organization

```
src/arch/        → x86_64 architecture (GDT, IDT, paging)
src/memory/      → Heap allocator and VM management
src/process/     → Task scheduler and process management
src/filesystem/  → VFS and filesystem drivers
src/io/          → Serial port and console drivers
src/sync/        → Spinlocks and semaphores
src/main.rs      → Kernel entry point
src/lib.rs       → Core kernel library
```

### Code Quality Tools

```bash
make fmt         # Format code with rustfmt
make fmt-check   # Check formatting without changes
make clippy      # Run clippy linter
make check       # Quick syntax check
```

### Testing

```bash
cargo test --lib         # Run unit tests
cargo test --all         # Run all tests
```

### Common Development Tasks

```bash
# Check code without building
cargo check

# Watch for changes and rebuild
cargo watch

# Generate documentation
cargo doc --no-deps --open
```

## Architecture Overview

### Bootloader Integration
MonarchOS uses the [Limine bootloader](https://github.com/limine-bootloader/limine), a modern protocol-based bootloader that handles firmware abstraction and provides a clean interface to kernel code. This allows us to focus on kernel development without worrying about legacy boot code.

### Memory Layout
```
0x0000000000000000 ┌─────────────────────────┐
                   │   User Space            │
                   │   (Future)              │
                   ├─────────────────────────┤
0xFFFF800000000000 │   Kernel Space          │
                   │   (Text, Data, BSS)     │
                   │   (Heap grows up)       │
                   │   (Stack grows down)    │
0xFFFFFFFFFFFFFFFF └─────────────────────────┘
```

### Core Subsystems

#### Architecture (arch/)
Processor-specific code for x86-64:
- **GDT**: Segmentation and privilege levels
- **IDT**: Exception and interrupt handling
- **Paging**: Virtual memory management

#### Memory (memory/)
Dynamic and virtual memory management:
- Heap allocator for runtime allocation
- Page table management
- Physical/virtual address mapping

#### Process (process/)
Multitasking and scheduling:
- Task/process structures
- Process scheduler
- Context switching

#### Filesystem (filesystem/)
Persistent storage:
- VFS abstraction layer
- ext2 filesystem driver
- Future: other filesystems

#### I/O (io/)
Hardware communication:
- Serial port (UART) for debugging
- Framebuffer console output
- Future: device drivers

#### Synchronization (sync/)
Thread safety primitives:
- Spinlocks (for kernel critical sections)
- Semaphores (for resource counting)

## Contributing

This is an educational OS project. Contributions are welcome! Areas needing work:

1. **Bootloader improvements**: Better limine integration
2. **Architecture code**: Complete GDT/IDT/paging implementations
3. **Memory management**: Improve allocator design
4. **Process scheduling**: Implement CFS or similar scheduler
5. **Filesystem**: Complete ext2 implementation
6. **Device support**: Add drivers for common hardware
7. **Documentation**: Improve code comments and guides
8. **Tests**: Add comprehensive test coverage

## Learning Resources

- **OS Development**
  - [OSDev.org](https://wiki.osdev.org/) - Comprehensive OS development wiki
  - [Writing an OS in Rust](https://github.com/phil-opp/blog_os) - Excellent tutorial series
  - [xv6 Operating System](https://github.com/mit-pdos/xv6-public) - Educational Unix-like OS

- **x86-64 Architecture**
  - [AMD64 Architecture Programmer's Manual](https://www.amd.com/en/technologies/x86)
  - [Intel x86-64 Documentation](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
  - [x86-64 ISA Reference](https://www.felixcloutier.com/x86/)

- **Rust Systems Programming**
  - [The Rust Book](https://doc.rust-lang.org/book/)
  - [Rustonomicon (Unsafe Rust)](https://doc.rust-lang.org/nomicon/)
  - [Embedded Rust Book](https://rust-embedded.github.io/book/)

- **Bootloader & Firmware**
  - [Limine Bootloader Documentation](https://github.com/limine-bootloader/limine)
  - [UEFI Specification](https://uefi.org/specifications)
  - [x86 Boot Process](https://wiki.osdev.org/Boot_Sequence)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Phil Oppermann](https://github.com/phil-opp) for the excellent "Writing an OS in Rust" blog
- [OSDev community](https://osdev.org/) for comprehensive resources
- Rust community for amazing systems programming tools and documentation

## Disclaimer

MonarchOS is an educational project. It is not recommended for production use without significant additional work on stability, security, and compatibility testing.
