# Running MonarchOS in QEMU

## Prerequisites

You need QEMU installed on your system to run the kernel.

### Install QEMU

**On Ubuntu/Debian:**
```bash
sudo apt-get install qemu-system-x86
```

**On Fedora/RHEL:**
```bash
sudo dnf install qemu-system-x86
```

**On macOS (with Homebrew):**
```bash
brew install qemu
```

**On Windows:**
Download from [qemu.org](https://www.qemu.org/download/) or use `choco install qemu` if using Chocolatey.

## Running the Kernel

### Option 1: Headless Mode (Recommended)

```bash
make run
```

This builds the kernel in release mode and runs it in QEMU without a display window. Output goes to stdio.

### Option 2: With Graphical Display

```bash
make run-gui
```

This runs QEMU with a graphical window so you can see the framebuffer output directly.

### Option 3: Manual Command

If you prefer to run QEMU manually:

```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/kernel \
    -m 512M \
    -serial stdio \
    -display none
```

### Option 4: Build Only (Without Running)

```bash
make build
```

This builds the kernel without running it. The binary is located at:
```
target/x86_64-unknown-none/release/kernel
```

## What to Expect

When you run the kernel, you should see:

1. **Kernel boots** via the limine bootloader
2. **Initialization** of:
   - GDT (Global Descriptor Table)
   - IDT (Interrupt Descriptor Table)
   - Paging and virtual memory
   - Heap allocator
3. **Framebuffer cleared** to black (if graphics available)
4. **Halt** - kernel enters infinite `hlt` loop

**Success indicator**: The kernel boots without panicking or triggering page faults.

## Troubleshooting

### "command not found: qemu-system-x86_64"

QEMU is not installed. Install it using the instructions above for your platform.

### "No such file or directory: target/x86_64-unknown-none/release/kernel"

The kernel hasn't been built yet. Run `make build` first, or just use `make run` which builds automatically.

### "could not read the boot disk" or "no valid configs in qemu"

This indicates an issue with the limine bootloader configuration or ISO creation. This is a known issue where QEMU's CD-ROM emulation may have difficulty reading the limine boot sectors.

**Possible causes:**
- CD-ROM emulation issues in your QEMU version
- Limine configuration syntax errors in `limine.conf`
- ISO creation process (xorriso configuration)

**Attempts at resolution:**
- Verified that `limine.conf` uses correct syntax for the limine protocol
- Tested with both limine v7.x and v9.x bootloader versions
- ISO structure verified to contain proper boot sectors

**Workaround:** Currently investigating direct kernel loading with QEMU's `-kernel` option, which requires PVH ELF Note support.

### Kernel hangs or shows black screen

This is normal behavior - the kernel initializes and halts (waits in a loop). There's no interactive shell yet.

### QEMU crashes or exits unexpectedly

Check for:
- Sufficient disk space (build artifacts can be large)
- Rust toolchain issues: `rustup update && rustup component add rust-src`
- Try a clean build: `make clean && make build`

## QEMU Options Explained

The default Makefile uses these QEMU options:

```
-kernel <file>      # Path to kernel binary
-m 512M              # Memory: 512 MB
-serial stdio        # Redirect serial port to stdout
-display none        # Run headless (no window)
```

You can add more QEMU options if needed:

```bash
# With debugging enabled
qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/kernel \
    -m 512M -serial stdio -display none -d cpu_reset,guest_errors

# With GDB debugging
qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/kernel \
    -m 512M -serial stdio -s -S
```

See [QEMU documentation](https://www.qemu.org/docs/master/system/index.html) for more options.
