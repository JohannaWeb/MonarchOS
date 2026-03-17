/// Physical frame allocator using bitmap

use crate::sync::spinlock::Spinlock;

const PAGE_SIZE: usize = 4096;
const MAX_PHYS_GB: usize = 64;
const TOTAL_FRAMES: usize = MAX_PHYS_GB * 1024 * 1024 * 1024 / PAGE_SIZE; // 16M frames
const BITMAP_WORDS: usize = TOTAL_FRAMES / 64; // 256K u64s = 2 MiB BSS

// Compile-time check
const _: () = assert!(BITMAP_WORDS * 64 == TOTAL_FRAMES);

pub struct FrameAllocator {
    bitmap: [u64; BITMAP_WORDS], // 1 = used, 0 = free
    free_frames: usize,
    next_search_word: usize, // next-fit hint
}

impl FrameAllocator {
    const fn new_empty() -> Self {
        FrameAllocator {
            bitmap: [!0u64; BITMAP_WORDS], // all used initially
            free_frames: 0,
            next_search_word: 0,
        }
    }

    #[allow(dead_code)]
    fn mark_free(&mut self, frame: u64) {
        let frame_idx = (frame / PAGE_SIZE as u64) as usize;
        let word_idx = frame_idx / 64;
        let bit_idx = frame_idx % 64;
        if word_idx < BITMAP_WORDS {
            let was_used = (self.bitmap[word_idx] & (1u64 << bit_idx)) != 0;
            self.bitmap[word_idx] &= !(1u64 << bit_idx);
            if was_used {
                self.free_frames += 1;
            }
        }
    }

    fn mark_used(&mut self, frame: u64) {
        let frame_idx = (frame / PAGE_SIZE as u64) as usize;
        let word_idx = frame_idx / 64;
        let bit_idx = frame_idx % 64;
        if word_idx < BITMAP_WORDS {
            let was_free = (self.bitmap[word_idx] & (1u64 << bit_idx)) == 0;
            self.bitmap[word_idx] |= 1u64 << bit_idx;
            if was_free {
                self.free_frames = self.free_frames.saturating_sub(1);
            }
        }
    }

    fn alloc_frame(&mut self) -> Option<u64> {
        // Search from next_search_word onward
        for start_word in 0..BITMAP_WORDS {
            let word_idx = (self.next_search_word + start_word) % BITMAP_WORDS;
            let word = self.bitmap[word_idx];

            // If word is not all 1s, it has a free frame
            if word != !0u64 {
                // Find first 0 bit (free frame)
                let bit_idx = (!word).trailing_zeros() as usize;
                self.bitmap[word_idx] |= 1u64 << bit_idx;
                self.free_frames = self.free_frames.saturating_sub(1);
                self.next_search_word = word_idx;

                let frame_idx = word_idx * 64 + bit_idx;
                let phys_addr = (frame_idx as u64) * PAGE_SIZE as u64;
                return Some(phys_addr);
            }
        }
        None
    }

    fn free_frame(&mut self, phys: u64) {
        let frame_idx = (phys / PAGE_SIZE as u64) as usize;
        let word_idx = frame_idx / 64;
        let bit_idx = frame_idx % 64;
        if word_idx < BITMAP_WORDS {
            let was_used = (self.bitmap[word_idx] & (1u64 << bit_idx)) != 0;
            self.bitmap[word_idx] &= !(1u64 << bit_idx);
            if was_used {
                self.free_frames += 1;
            }
        }
    }
}

static FRAME_ALLOCATOR: Spinlock<FrameAllocator> = Spinlock::new(FrameAllocator::new_empty());

pub fn init() {
    let mut alloc = FRAME_ALLOCATOR.lock();
    let boot = crate::boot_info::get();

    // Mark all USABLE regions as free
    for entry in boot.memory_map {
        use limine::memory_map::EntryType;
        if entry.entry_type == EntryType::USABLE {
            let start_frame = (entry.base / PAGE_SIZE as u64) as usize;
            let end_frame = start_frame + (entry.length / PAGE_SIZE as u64) as usize;

            for frame_idx in start_frame..end_frame {
                let word_idx = frame_idx / 64;
                let bit_idx = frame_idx % 64;
                if word_idx < BITMAP_WORDS {
                    alloc.bitmap[word_idx] &= !(1u64 << bit_idx);
                }
            }
            alloc.free_frames += end_frame - start_frame;
        }
    }

    // Re-mark kernel image pages as used
    extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }

    let kernel_virt_start = &raw const __kernel_start as u64;
    let kernel_virt_end = &raw const __kernel_end as u64;

    let kernel_phys_start = kernel_virt_start - boot.kernel_virt_base + boot.kernel_phys_base;
    let kernel_phys_end = kernel_virt_end - boot.kernel_virt_base + boot.kernel_phys_base;

    let start_frame = (kernel_phys_start / PAGE_SIZE as u64) as usize;
    let end_frame = (kernel_phys_end / PAGE_SIZE as u64) as usize;

    for frame_idx in start_frame..end_frame {
        let word_idx = frame_idx / 64;
        let bit_idx = frame_idx % 64;
        if word_idx < BITMAP_WORDS {
            let was_free = (alloc.bitmap[word_idx] & (1u64 << bit_idx)) == 0;
            alloc.bitmap[word_idx] |= 1u64 << bit_idx;
            if was_free {
                alloc.free_frames = alloc.free_frames.saturating_sub(1);
            }
        }
    }

    // Mark frame 0 (null page) as used
    alloc.mark_used(0);
}

pub fn alloc_frame() -> Option<u64> {
    FRAME_ALLOCATOR.lock().alloc_frame()
}

pub fn free_frame(phys: u64) {
    FRAME_ALLOCATOR.lock().free_frame(phys);
}

pub fn free_frame_count() -> usize {
    FRAME_ALLOCATOR.lock().free_frames
}
