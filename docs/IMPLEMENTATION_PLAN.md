# MonarchOS Implementation Plan

This plan turns `docs/LINUS_REVIEW.md` into ordered engineering work. Check items
off as they are completed. Do not start process scheduling, syscalls, userspace,
or POSIX work until the foundation checklist at the end is complete.

## Phase 0: Repository And Documentation Hygiene

Status: partially complete.

### Tasks

- [x] Move generated boot artifacts out of the repo root.
- [x] Keep Limine checkout and generated ISO staging ignored.
- [ ] Update README claims so they match actual implemented behavior.
- [ ] Remove or reword claims that GDT, IDT, serial logging, console behavior,
  tests, and synchronization are complete.
- [ ] Track `src/terminal.rs` intentionally if it is part of the kernel.

### Acceptance Criteria

- [ ] `git status --short` shows no tracked generated ISO, kernel binary, qcow2,
  Limine checkout, or ISO staging changes after a build.
- [ ] README "Implemented" section contains only behavior that exists in code.
- [x] README links this plan and `docs/RUNNING.md`.

## Phase 1: Boot And Fault Handling

This is the first real implementation phase. Without reliable traps, every other
subsystem is harder to debug.

### 1. Implement GDT

Files:

- `src/arch/x86_64/gdt.rs`
- `src/arch/x86_64/mod.rs`

Tasks:

- [ ] Define kernel code and data segment descriptors.
- [ ] Add a TSS with a known-good interrupt stack table entry for double faults.
- [ ] Load the GDT during `arch::init()`.
- [ ] Reload segment registers after loading the GDT.

Acceptance criteria:

- [ ] `arch::init()` installs a kernel-owned GDT.
- [ ] The code no longer relies on Limine's inherited GDT.
- [ ] Double-fault stack is defined before IDT double-fault handler is enabled.

### 2. Implement IDT

Files:

- `src/arch/x86_64/idt.rs`
- `src/arch/x86_64/mod.rs`
- `src/io/serial.rs`

Tasks:

- [ ] Define IDT entry and pointer structures.
- [ ] Install handlers for divide error, breakpoint, invalid opcode, general
  protection fault, page fault, and double fault.
- [ ] Print exception details to serial output and halt cleanly.
- [ ] Load IDT during `arch::init()` after GDT setup.

Acceptance criteria:

- [ ] Triggering a known exception prints a useful diagnostic instead of
  silently rebooting.
- [ ] Page fault handler reports fault address, error code, RIP, CS, and RFLAGS.
- [ ] Double fault uses the dedicated IST stack.

## Phase 2: Debug I/O And Init Honesty

### 3. Implement Serial Output

Files:

- `src/io/serial.rs`
- `src/io/mod.rs`

Tasks:

- [ ] Initialize COM1.
- [ ] Implement blocking byte writes.
- [ ] Add minimal `core::fmt::Write` support for serial logging.
- [ ] Use serial logging in panic and exception paths.

Acceptance criteria:

- [ ] `make run` shows kernel-owned serial output, not only Limine messages.
- [ ] Panic and exception handlers can log without framebuffer availability.

### 4. Remove Decorative Init Stubs

Files:

- `src/lib.rs`
- `src/arch/x86_64/gdt.rs`
- `src/arch/x86_64/idt.rs`
- `src/memory/manager.rs`
- `src/process/mod.rs`

Tasks:

- [ ] Delete init calls that do not perform real initialization.
- [ ] Or implement the init function in the same change.
- [ ] Rename smoke-test functions where they are probes rather than
  initialization.

Acceptance criteria:

- [ ] Every function called by `monarch::init()` changes machine or kernel state
  in a meaningful way.
- [ ] Empty TODO init functions are not part of the boot path.

## Phase 3: Memory Safety Foundation

### 5. Fix Heap Allocator Metadata

Files:

- `src/memory/heap.rs`

Tasks:

- [ ] Remove byte-by-byte `find_block_header` scanning.
- [ ] Store exact header metadata immediately before the returned user pointer,
  or use an equivalent O(1) back-pointer scheme.
- [ ] Use the `Layout` passed to `dealloc`.
- [ ] Replace the magic `+ 8` split threshold with a named minimum block size.
- [ ] Ensure every free-list node has enough space for `FreeBlock`.

Acceptance criteria:

- [ ] `dealloc` finds its block header without scanning user memory.
- [ ] Small and highly aligned allocations cannot corrupt adjacent blocks.
- [ ] Allocator tests cover alignment, split/no-split cases, and free/reuse
  paths.

### 6. Replace `static mut BOOT_INFO`

Files:

- `src/boot_info.rs`
- `src/sync/`

Tasks:

- [ ] Replace direct `static mut` with a one-shot initialization primitive.
- [ ] Make repeated initialization fail loudly.
- [ ] Keep `get()` safe only if the synchronization primitive actually
  guarantees initialized immutable access.

Acceptance criteria:

- [ ] `make check` no longer emits `static_mut_refs` from `boot_info.rs`.
- [ ] Calling `set()` twice panics or returns an explicit error.

### 7. Rework Frame Allocator Sizing

Files:

- `src/memory/allocator.rs`
- `src/boot_info.rs`

Tasks:

- [ ] Stop hardcoding a 64 GiB physical memory bitmap.
- [ ] Size allocator metadata from Limine's memory map.
- [ ] Reserve the low 1 MiB, not only frame zero.
- [ ] Prefer range operations over per-frame loops where possible.

Acceptance criteria:

- [ ] Metadata size scales with detected physical memory.
- [ ] Frames above the old 64 GiB ceiling are not silently ignored.
- [ ] Low memory is reserved before general allocation starts.

## Phase 4: Synchronization Correctness

### 8. Fix Semaphore Acquire

Files:

- `src/sync/semaphore.rs`

Tasks:

- [ ] Replace decrement-check-undo logic with a CAS loop.
- [ ] Keep release simple, but define overflow behavior.
- [ ] Add host-testable unit tests if possible.

Acceptance criteria:

- [ ] `acquire()` never transiently exposes `u32::MAX` as available capacity.
- [ ] Contended acquire only succeeds after a successful compare-exchange.

### 9. Add IRQ-Safe Spinlocks

Files:

- `src/sync/spinlock.rs`
- `src/arch/x86_64/`

Tasks:

- [ ] Add local interrupt save/restore helpers.
- [ ] Add `lock_irqsave()` or make kernel spinlocks save IRQ state by default.
- [ ] Restore prior interrupt state on guard drop.
- [ ] Consider a `Drop` implementation for non-static `Spinlock<T>`.

Acceptance criteria:

- [ ] Locks used from interrupt-capable paths cannot deadlock against same-CPU
  ISRs.
- [ ] Guard drop restores the exact previous interrupt-enabled state.

## Phase 5: Build Configuration Cleanup

### 10. Resolve Target And Relocation Configuration

Files:

- `.cargo/config.toml`
- `x86_64-monarch.json`
- `linker.ld`

Tasks:

- [ ] Either delete `x86_64-monarch.json` if unused, or switch Cargo to use it.
- [ ] If using the custom target, set relocation behavior to match the PIE
  kernel.
- [ ] Keep linker script dynamic relocation sections only if the loader path
  needs them.

Acceptance criteria:

- [ ] There is one source of truth for the kernel target.
- [ ] Target spec, rustflags, and linker script agree on PIE/static behavior.

### 11. Remove Duplicate Compiler Intrinsics

Files:

- `src/main.rs`
- `.cargo/config.toml`

Tasks:

- [ ] Keep `compiler-builtins-mem`.
- [ ] Delete local `memcpy`, `memmove`, `memset`, and `memcmp` definitions.

Acceptance criteria:

- [ ] Release build links without local duplicate intrinsic symbols.
- [ ] Memory intrinsic symbols come from `compiler_builtins`.

### 12. Fix Or Remove `make test`

Files:

- `Makefile`
- `README.md`

Tasks:

- [ ] Either implement a no-std/QEMU test runner, or remove the `test` target.
- [ ] Remove or update README test commands to match the real test path.

Acceptance criteria:

- [ ] Every documented command works.
- [ ] `make test` is not present unless it provides a useful kernel test path.

## Phase 6: Terminal And Shell Separation

Files:

- `src/terminal.rs`
- `src/io/console.rs`
- future keyboard driver module

Tasks:

- [ ] Split framebuffer console, PS/2 keyboard polling, and shell command
  handling.
- [ ] Replace clear-on-scroll with actual scrolling or a line ring buffer.
- [ ] Do not clear boot diagnostics when entering the shell.
- [ ] Use keyboard-controller reboot only as a fallback.

Acceptance criteria:

- [ ] Terminal output scrolls without deleting all prior output.
- [ ] Shell input logic is separate from framebuffer rendering.
- [ ] Boot logs remain visible or available after shell startup.

## Suggested Commit Order

- [ ] Documentation honesty and generated-artifact cleanup.
- [ ] Serial logging.
- [ ] GDT.
- [ ] IDT and exception diagnostics.
- [ ] Heap allocator metadata fix.
- [ ] Semaphore CAS loop.
- [ ] IRQ-safe spinlocks.
- [ ] Boot info one-shot primitive.
- [ ] Target/linker cleanup and duplicate intrinsic removal.
- [ ] Test target decision.
- [ ] Terminal split.

## Foundation Definition Of Done

The foundation is ready for scheduler work only when every item below is checked:

- [ ] Faults print deterministic diagnostics instead of rebooting silently.
- [ ] Heap allocation and deallocation do not use heuristic metadata discovery.
- [ ] Synchronization primitives are safe for interrupt-aware kernel code.
- [ ] Boot info access is one-shot and does not rely on direct `static mut` refs.
- [ ] Build configuration has no contradictory target or relocation settings.
- [ ] README and Makefile only document commands and features that work.
