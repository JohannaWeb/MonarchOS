use core::sync::atomic::{AtomicU32, Ordering};

/// Counting semaphore for kernel synchronization
pub struct Semaphore {
    count: AtomicU32,
}

impl Semaphore {
    pub const fn new(initial: u32) -> Self {
        Self {
            count: AtomicU32::new(initial),
        }
    }

    pub fn acquire(&self) {
        while self.count.fetch_sub(1, Ordering::Acquire) == 0 {
            self.count.fetch_add(1, Ordering::Release);
            core::hint::spin_loop();
        }
    }

    pub fn release(&self) {
        self.count.fetch_add(1, Ordering::Release);
    }
}
