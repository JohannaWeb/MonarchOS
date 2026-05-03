# MonarchOS — Code Review

**Reviewer:** Linus (in spirit)
**Date:** 2026-05-03
**Verdict:** Stop lying to yourself in the README and start fixing the actual bugs.

---

## TL;DR

You have a kernel that boots, prints green text on a framebuffer, polls the keyboard
in a busy loop, and pretends it has GDT, IDT, and exception handling. It doesn't.
The heap allocator looks competent at a glance and falls apart the moment you read
`find_block_header`. The semaphore has a real race. The spinlock will deadlock the
day you wire up interrupts. And there's enough magic numbers, copy-paste, and
"TODO" stubs masquerading as "done" that I'd reject this on the mailing list before
finishing my coffee.

The good news: this is fixable. The bad news: most of it has to be fixed before
you can even start on a scheduler, never mind "POSIX compliance".

---

## 1. You are lying in your own README

`README.md:78-86` cheerfully lists as **implemented**:

> - Spinlock and semaphore synchronization primitives
> - **Basic exception handling stub**
> - Proper bare-metal configuration

And `memory/project_monarchos.md` says:

> 2. GDT/IDT initialization (done - Milestone 2)

Now let's read the actual code:

```rust
// src/arch/x86_64/gdt.rs
pub fn init() {
    // TODO: Initialize GDT
    // Set up kernel code/data segments, TSS for exception handling
}
```

```rust
// src/arch/x86_64/idt.rs
pub fn init() {
    // TODO: Initialize IDT
    // Set up exception handlers, hardware interrupt handlers
}
```

That is not "done". That is not "stub". That is *nothing*. You are running on
whatever GDT Limine left in CR0/CS/SS, and the IDT is whatever Limine handed you,
which means the *first* page fault, GP fault, or divide-by-zero **triple-faults
the CPU and reboots the box**. You have no exception handlers because you have no
IDT because you didn't write one.

Fix it or fix the documentation. Right now the documentation is dishonest, and
nothing destroys the credibility of an OS project faster than a fake TODO list.

`io/serial.rs` and `io/console.rs` have the same disease — `init()` is empty,
`write_byte` literally does `let _ = byte`. The README claims serial logging
works. It does not. `make run` only sees stdout because Limine's verbose mode is
echoing on COM1, not because *your* serial driver works.

**Action:** Either implement these, or remove "✅" from the README until they work.
Stop checking off boxes you haven't earned.

---

## 2. The heap allocator's `find_block_header` is the worst code in the kernel

`src/memory/heap.rs:133-160` — read it and weep:

```rust
unsafe fn find_block_header(&self, alloc_ptr: *mut u8) -> *mut FreeBlock {
    let mut search_addr = alloc_ptr as usize;
    while search_addr >= self.heap_start {
        let potential_header = search_addr as *const BlockHeader;
        let size = (*potential_header).size;
        if size > 0 && size <= self.heap_max_end - self.heap_start {
            // … "is this a real header?"
            ...
        }
        search_addr = search_addr.saturating_sub(1);
    }
}
```

You are walking *backwards one byte at a time* from the user pointer trying to
**guess** where the block header lives, using "is this number plausible?" as your
sanity check. With `heap_max_end - heap_start = 1 GiB`, *almost any* `usize` value
between 1 and 1 GiB passes the check. The first 8 bytes of any user allocation
that happens to encode a value in `[1, 1 GiB]` will be mistaken for a header.

This is not a hash table. It's not a bloom filter. It's a *heuristic*, and it's
running in `dealloc`. Every freed pointer rolls dice against the contents of
your own heap. When this code corrupts memory, it will do so silently, and you
will spend three weeks finding it.

**Why it exists at all:** because you wanted to support arbitrary alignment with
padding *before* the user pointer (`heap.rs:71-117`). The aligned user start can
be any number of bytes after the header, so you can't just go `ptr - 8` to find
the header. So instead of solving that problem properly, you decided to scan.

**Fix:** the standard textbook answer is to store a small back-pointer (or the
exact header offset) in the **8 bytes immediately before** the aligned user
pointer. Allocator does:

```
[ header | padding | back_ptr | user_data ]
                              ^
                              aligned_start
```

`back_ptr = aligned_start - 8` stores the offset to the header. `dealloc` reads
those 8 bytes and goes straight to the header. O(1), no heuristic, no UB.

Or — even simpler — over-align everything to the larger of `align_of::<usize>()`
and the requested alignment, and put the header at `aligned_start - sizeof(header)`.
You already pay the alignment cost; might as well get correctness for it.

While you're in there, also: `dealloc` *receives the original `Layout`* (`heap.rs:258`,
parameter is `_layout`). You're throwing away ground truth and then heuristically
reconstructing it. That is masochism, not engineering. Use the layout.

---

## 3. The semaphore has a real race

`src/sync/semaphore.rs:15-20`:

```rust
pub fn acquire(&self) {
    while self.count.fetch_sub(1, Ordering::Acquire) == 0 {
        self.count.fetch_add(1, Ordering::Release);
        core::hint::spin_loop();
    }
}
```

Walk through it with two threads, count = 1:
- Thread A: `fetch_sub` returns 1, count is now 0. Enter critical section. Good.
- Thread B: `fetch_sub` returns 0, count is now `u32::MAX` (wraparound).
  Wait — that's not even checked. The condition is `== 0`, so B's `fetch_sub`
  returned 0, B *enters the loop body*, restores count via `fetch_add` to ... 0?
  No, count was `u32::MAX`, so `fetch_add(1)` wraps back to 0. OK in this 2-thread
  toy case.
- Now Thread C arrives while B is between `fetch_sub` and `fetch_add`. Count is
  `u32::MAX`. C's `fetch_sub` returns `u32::MAX`, which is *not* zero, so C
  considers itself successful and **enters the critical section alongside A**.

That is a textbook semaphore race. You can't do "decrement, check, undo" without
a CAS loop. The correct loop is:

```rust
loop {
    let c = self.count.load(Acquire);
    if c == 0 { core::hint::spin_loop(); continue; }
    if self.count.compare_exchange_weak(c, c - 1, Acquire, Relaxed).is_ok() {
        return;
    }
}
```

Same thing for `release` is fine.

This kind of thing is exactly why you don't roll your own primitives until you've
read Preshing, the C++ memory model paper, and Paul McKenney. There's a reason
`std::sync::Semaphore` is hard to write.

---

## 4. The spinlock will deadlock the moment you wire up interrupts

`src/sync/spinlock.rs:19-25`:

```rust
pub fn lock(&self) -> SpinlockGuard<'_, T> {
    while self.locked.compare_exchange(false, true, Acquire, Relaxed).is_err() {
        core::hint::spin_loop();
    }
    SpinlockGuard { lock: self }
}
```

This is a *userspace* spinlock pretending to be a kernel spinlock. Kernel
spinlocks must disable interrupts on the local CPU while held, otherwise:

1. Thread T grabs `FRAME_ALLOCATOR.lock()`.
2. Timer interrupt fires on the same CPU.
3. ISR allocates a frame → `FRAME_ALLOCATOR.lock()` again on the same CPU.
4. CAS fails forever. Single-CPU deadlock.

You don't have interrupts yet, so you haven't hit this — but you will, the day
you implement IDT. Add an `irqsave` flavor that does `pushf; cli` on entry and
restores on drop:

```rust
pub fn lock_irqsave(&self) -> IrqSpinlockGuard<'_, T> { ... }
```

Or commit to the idea that *all* spinlocks save IRQ state. Either way, this needs
to be designed in *now*, not retrofitted after you have a scheduler and three
deadlocks you can't reproduce.

Also: no `Drop` impl for `Spinlock<T>` — fine for `'static` use, but if anyone
ever puts a `Spinlock<Box<T>>` in a non-static context, the inner `T` never gets
its destructor called when the Spinlock is dropped without being locked. Edge
case, but write it down.

---

## 5. PIE vs static relocation: pick one

`x86_64-monarch.json:14`:
```json
"relocation-model": "static",
```

`.cargo/config.toml:3`:
```
rustflags = ["-C", "link-args=-T linker.ld", "-C", "link-args=-pie", ...]
```

`linker.ld:31-37`:
```
.hash : { *(.hash) } :rodata
.gnu.hash : { *(.gnu.hash) } :rodata
.dynsym : { *(.dynsym) } :rodata
.dynstr : { *(.dynstr) } :rodata
.rela.dyn : { *(.rela.dyn) } :rodata
.rela.plt : { *(.rela.plt) } :rodata
```

`linker.ld:50-52`:
```
.dynamic : {
    *(.dynamic)
} :data :dynamic
```

So your custom target says **static**, your linker flags say **PIE**, and your
linker script preserves all the **dynamic relocation** sections so the kernel
can be relocated at load time. You are building a PIE kernel and lying about it
in the target spec. That x86_64-monarch.json file is unused anyway —
`.cargo/config.toml:2` builds against `x86_64-unknown-none`. So either:

- **Delete `x86_64-monarch.json`** if it's dead code. It is.
- **Use it** by switching `target = "x86_64-monarch.json"` and fix `relocation-model`
  to `"pic"` so the target spec is honest.

Right now you have two configs that disagree, and the wrong one is "winning" by
being unused. That is a maintenance bomb.

---

## 6. Compiler intrinsics: pick one source

`Cargo.toml`/`.cargo/config.toml` enables `compiler-builtins-mem`:

```toml
build-std-features = ["compiler-builtins-mem"]
```

That feature provides `memcpy`, `memmove`, `memset`, `memcmp` from
`compiler_builtins`. Then `src/main.rs:17-52` defines the **same four symbols**
yourself with `#[no_mangle]`.

You either get a duplicate-symbol error (in which case your build is broken
right now and you don't know it because LLD is being lenient with weak
symbols), or your hand-rolled versions silently override the optimized
intrinsic ones. Either way it's wrong.

**Fix:** delete the four functions from `main.rs`. `compiler-builtins-mem` is
already doing the job and its versions are SIMD-optimized. Yours are byte loops.

---

## 7. `static mut BOOT_INFO` — 2018 called

`src/boot_info.rs:10`:

```rust
static mut BOOT_INFO: Option<BootInfo> = None;
```

Direct `static mut` access is being deprecated in Rust 2024 and is already a
lint in 2021 with `static_mut_refs`. You wrote this in 2026. There is no excuse.

Use `OnceLock`-style init via a `Spinlock<Option<BootInfo>>`, or `AtomicPtr`,
or — given you're explicitly single-init-once — `core::cell::OnceCell` wrapped
in a sync wrapper. Or `spin::Once`. Anything but `static mut` with two unsafe
APIs around it pretending the safety is handled.

Bonus: `boot_info::get()` is not `unsafe` but reads `static mut` without
synchronization. Today it works because you only call it after `set()` and
before any concurrency. The day you have an SMP-aware kernel, this will be
the kind of bug that takes a week to find.

---

## 8. The frame allocator is wasteful and wrong-by-default

`src/memory/allocator.rs:5-8`:

```rust
const MAX_PHYS_GB: usize = 64;
const TOTAL_FRAMES: usize = MAX_PHYS_GB * 1024 * 1024 * 1024 / PAGE_SIZE; // 16M frames
const BITMAP_WORDS: usize = TOTAL_FRAMES / 64; // 256K u64s = 2 MiB BSS
```

You hardcoded **2 MiB of BSS** for a bitmap covering 64 GiB of phys RAM
*regardless of how much RAM the box actually has*. On a 512 MiB QEMU instance
that's fine — wasteful, but fine. On a 1 TiB server it silently fails to
account for frames above 64 GiB. Both directions are wrong.

The right answer is a buddy allocator or a bitmap sized at runtime from the
memory map, allocated out of the bootloader-reserved low memory. This is
"first kernel project" territory, but you're calling this "Unix-like with full
POSIX compliance" — get it right.

`init()` at line 93-144 also does linear scans over every memory map entry to
flip bits one at a time. Use `memset` over whole bitmap words for any frame
range that's word-aligned. You'll thank yourself when boot time matters.

`alloc.mark_used(0)` at line 143 — fine, but the *entire* low 1 MiB should be
reserved (BIOS, IVT, BDA, EBDA). Treating just frame 0 as off-limits is a
bug waiting to happen the moment something tries to DMA into 0x500.

---

## 9. The heap allocator splitting math has off-by-ones and a magic 8

`src/memory/heap.rs:89`:

```rust
if block_size > total_used + BLOCK_HEADER_SIZE + 8 {
    // Split the block...
```

What is the `+ 8`? Minimum useful free block? `sizeof(FreeBlock)` is
`size + next` = 16 bytes on x86_64, not 8. If you're trying to ensure the
remainder can hold a `FreeBlock`, the threshold should be `sizeof(FreeBlock) =
16`. If you're trying to ensure it can hold a `BlockHeader` plus alignment,
say so. Right now it's a magic number and it's wrong.

Also: `total_used = BLOCK_HEADER_SIZE + padding + alloc_size`. If
`alloc_align` is 64, `padding` can be up to 63 bytes. If `alloc_size` is small,
total_used can be smaller than `sizeof(FreeBlock) = 16`, which means when the
block is later freed and re-inserted into the free list, **`(*free_block).next`
is being written past the end of the allocation footprint into a neighbor**.
Memory corruption.

Minimum block size must be `max(sizeof(FreeBlock), header + min_user_size)`.
You don't enforce this anywhere.

---

## 10. The terminal is a toy and the README pretends it's a console

`src/terminal.rs:108`:

```rust
if self.cursor_y + 8 * self.scale > self.height {
    // Scroll not implemented, just wrap to top for now
    self.clear();
}
```

When you scroll past the bottom of the screen, you **wipe the entire
terminal**. That's not "wrap to top" — that's "delete everything the user typed".
This needs a real scroll (memmove the framebuffer up by one row of glyphs)
or, more sensibly, switch to a ring buffer of lines and re-render.

`get_char()` at line 125 polls `0x60`/`0x64` in a tight loop with no
interrupt-driven I/O. The dedup logic at line 151:

```rust
if scancode == last_scancode {
    continue;
}
```

is wrong — repeat-on-hold legitimately delivers the same scancode multiple
times. You're getting away with it only because PS/2 sends a break code (0xAE
for 'a' release) between repeats during normal typing. The day a key is held
without release codes (focus loss in QEMU, USB HID emulation, etc.) you lose
keystrokes.

`run_shell()` calls `self.clear()` at the top, so every diagnostic message
printed during `_start()` is wiped before the user sees it. If something
goes wrong during init, you can't see it — the shell ate the boot log.

`reboot` busy-waits on the keyboard controller, which is the 1980s answer.
The modern answer is `outb(0xCF9, 0x06)` (PCI reset) or a triple fault. Use
the old way as fallback only.

And finally — this whole file isn't an OS terminal, it's a single hardcoded
struct stuffed into `lib.rs`. There's no separation between "framebuffer
console" and "shell" and "PS/2 driver". Three responsibilities crammed into
one file. Split it.

---

## 11. Build artifacts in git

```
M iso_root/boot/kernel
M kernel.iso
M src/lib.rs
?? src/terminal.rs
```

You are tracking a 4.5 MiB ISO image and a kernel binary in git. Your
`.gitignore` is **8 bytes**. This repo will hit 100 MiB of garbage history
inside a month.

Add to `.gitignore`:

```
/target/
/iso_root/
/limine/
*.iso
*.qcow2
kernel.gz
```

Then `git rm --cached kernel.iso iso_root/boot/kernel kernel.gz disk.qcow2`
**in a separate commit** so reviewers can see the cleanup. And commit
`src/terminal.rs` if you actually want it tracked, instead of leaving it as
"??" forever.

---

## 12. `cargo test` will not work and you have a `make test` target for it

`Makefile:53`:

```
test:
	cargo test
```

This is a `no_std`, `no_main`, custom-target kernel. `cargo test` requires the
host target by default and the standard test harness, neither of which you
have. Running `make test` produces a wall of red. Either:

- Set up a custom test runner with `#![test_runner = "..."]` and
  `#![reexport_test_harness_main = "..."]`, run inside QEMU and read results
  via serial.
- Or delete the `test` target from the Makefile and stop pretending you have
  tests.

The `cargo test --lib` line in the README is also a lie.

---

## 13. The init story is a sequence of empty TODOs

```rust
// src/lib.rs
pub fn init() {
    arch::init();   // calls gdt::init() (TODO), idt::init() (TODO), paging::init() (does work)
    memory::init(); // allocator::init (real), manager::init (TODO), heap::init (real)
    process::init();// TODO
}
```

Out of 7 init functions called, **3 are completely empty**, **1 is "// Future:"**,
and only **3 do real work**. Half your init sequence is decoration. If a function
doesn't do anything, *don't call it*. Adding it as a stub for future work is a
common but bad habit — it makes every init trace look longer and more impressive
than it is, and it makes failures harder to localize.

Delete the stubs. Add them when you implement them.

---

## 14. Stylistic and minor

- `arch::halt()` (`arch/mod.rs:14-21`) is dead code. `main.rs:172` defines its
  own `halt()`. Pick one and delete the other.
- `BOOT_INFO`'s `get()` returns a `&'static BootInfo` from a `static mut`. The
  lifetime is a lie if anyone ever calls `set()` twice. Make `set()` panic on
  re-init.
- `PageTable::new()` (`paging.rs:51-57`) is `fn new()`, not `pub`. Used only
  via `core::ptr::write` in `map_page`. Fine, but keep an eye on it.
- `paging::init()` does an `assert!` at lines 190-204 and a `read_volatile` of
  the HHDM base. That's a smoke test, not initialization. Either rename it
  `paging::probe()` or actually do something (e.g., reload CR3 with a kernel-owned
  PML4 instead of trusting Limine's).
- The PML4 you use is **Limine's**. You never copy it to a kernel-owned PML4.
  The day Limine reclaims its bootloader-reclaimable regions, your PML4
  vanishes with them. Build your own PML4 early.
- `terminal.rs:79`: `BASIC_LEGACY[c as usize]` — bounds-checked Rust, but
  guarded only by `(c as u32) < 128`. Fine, but a comment explaining why you
  don't need to handle higher Unicode would help readers.
- `Cargo.toml`: `edition = "2021"`, but you're on nightly and using
  `&raw const` (allocator.rs:121) which is stable in 1.82+. Move to
  `edition = "2024"` and clean up the `static mut` warnings while you're at it.

---

## What to do, in order

1. **Stop lying.** Update README and project memory to reflect actual state.
   Mark GDT/IDT/serial/console as TODO, not "✅".
2. **Implement GDT and IDT.** Without these, you can't trap exceptions, you
   can't do syscalls, you can't preempt. Everything else is castles in the air.
3. **Fix the heap allocator.** Replace `find_block_header` with a back-pointer
   approach. Fix the magic `+ 8`. Enforce a minimum block size of
   `sizeof(FreeBlock)`.
4. **Fix the semaphore.** CAS loop, not decrement-and-undo.
5. **Add IRQ-safe spinlocks** before you implement interrupt handlers.
6. **Resolve the PIE/static contradiction.** Delete `x86_64-monarch.json` or
   use it.
7. **Delete the duplicate `memcpy`/`memmove`/`memset`/`memcmp`** from
   `main.rs` — `compiler-builtins-mem` already provides them.
8. **Replace `static mut BOOT_INFO`** with a proper one-shot sync primitive.
9. **Get the artifacts out of git.** Fix `.gitignore`, untrack the binaries.
10. **Either fix `make test` or delete it.**

Once those ten are done, *then* you can talk about a process scheduler. Not
before. There is no point implementing CFS on top of an allocator that
heuristically guesses where its own block headers live.

---

## Final word

The architecture skeleton is reasonable. The Limine integration is correct.
The paging code is more careful than I'd expect from an early-stage hobby
kernel — the compile-time `size_of`/`align_of` asserts (paging.rs:4-6)
are exactly the right instinct. You've clearly read enough OSDev to know
the shape of the answer.

But the gap between "looks like a kernel" and "is a kernel" is enormous,
and most of the gap is in the unglamorous bits: real interrupt handling,
real synchronization, real allocator correctness, honest documentation. You
have skipped almost all of that and skipped straight to a green-on-black
shell that prints "Welcome to MonarchOS!" — which looks impressive in a
screenshot and falls over the moment it sees a page fault.

Build the boring parts. The fun parts come after.

— L.
