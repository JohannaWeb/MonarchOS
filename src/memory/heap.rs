/// Kernel heap allocator using linked-list free-list

use core::alloc::{GlobalAlloc, Layout};
use crate::sync::spinlock::Spinlock;

const PAGE_SIZE: usize = 4096;
const HEAP_START: usize = 0xFFFF_C000_0000_0000;
const HEAP_INITIAL_PAGES: usize = 1024; // 4 MiB
const HEAP_MAX_PAGES: usize = 262144; // 1 GiB max
const BLOCK_HEADER_SIZE: usize = 8; // size field only

#[repr(C)]
struct BlockHeader {
    size: usize,
}

struct FreeBlock {
    header: BlockHeader,
    next: *mut FreeBlock,
}

struct LinkedListHeap {
    free_list: *mut FreeBlock,
    heap_start: usize,
    heap_current_end: usize,
    heap_max_end: usize,
}

impl LinkedListHeap {
    const fn new(start: usize) -> Self {
        Self {
            free_list: core::ptr::null_mut(),
            heap_start: start,
            heap_current_end: start,
            heap_max_end: start + HEAP_MAX_PAGES * PAGE_SIZE,
        }
    }

    /// Allocate using first-fit strategy
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let alloc_size = layout.size();
        let alloc_align = layout.align();

        // Account for header and padding
        let total_size = BLOCK_HEADER_SIZE + alloc_size + (alloc_align - 1);

        // Try to find a suitable free block
        let mut current = self.free_list;
        let mut prev: *mut FreeBlock = core::ptr::null_mut();

        while !current.is_null() {
            let block = &*current;
            if block.header.size >= total_size {
                // Found a suitable block
                return self.allocate_from_block(current, prev, alloc_size, alloc_align);
            }
            prev = current;
            current = block.next;
        }

        // No suitable block found; try to grow heap
        if self.grow_heap(total_size).is_ok() {
            // After growing, recursively try allocation again
            return self.alloc(layout);
        }

        // Out of memory
        core::ptr::null_mut()
    }

    unsafe fn allocate_from_block(
        &mut self,
        block_ptr: *mut FreeBlock,
        prev_ptr: *mut FreeBlock,
        alloc_size: usize,
        alloc_align: usize,
    ) -> *mut u8 {
        let block = &mut *block_ptr;
        let block_start = block_ptr as usize;
        let block_size = block.header.size;

        // Calculate aligned user pointer (after header)
        let user_start = block_start + BLOCK_HEADER_SIZE;
        let aligned_start = (user_start + alloc_align - 1) & !(alloc_align - 1);
        let padding = aligned_start - user_start;

        let total_used = BLOCK_HEADER_SIZE + padding + alloc_size;

        if block_size > total_used + BLOCK_HEADER_SIZE + 8 {
            // Split the block: we keep the remainder as a free block
            let remainder_start = block_start + total_used;
            let remainder_size = block_size - total_used;

            let remainder = remainder_start as *mut FreeBlock;
            (*remainder).header.size = remainder_size;
            (*remainder).next = block.next;

            // Update the original block size
            block.header.size = total_used;

            // Update free list pointers
            if !prev_ptr.is_null() {
                (*prev_ptr).next = remainder;
            } else {
                self.free_list = remainder;
            }
        } else {
            // Use the entire block; remove it from free list
            if !prev_ptr.is_null() {
                (*prev_ptr).next = block.next;
            } else {
                self.free_list = block.next;
            }
        }

        aligned_start as *mut u8
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let block_ptr = self.find_block_header(ptr);
        if block_ptr.is_null() {
            return;
        }

        // Insert back into free list in address order and coalesce
        self.insert_free_block(block_ptr as usize);
    }

    unsafe fn find_block_header(&self, alloc_ptr: *mut u8) -> *mut FreeBlock {
        // Search backwards from the pointer to find the header
        let mut search_addr = alloc_ptr as usize;

        while search_addr >= self.heap_start {
            let potential_header = search_addr as *const BlockHeader;
            let size = (*potential_header).size;

            // Sanity check: size should be reasonable
            if size > 0 && size <= self.heap_max_end - self.heap_start {
                // Check if this is the correct header for our pointer
                let user_start = search_addr + BLOCK_HEADER_SIZE;
                let block_start = search_addr;
                let block_end = search_addr + size;

                if (alloc_ptr as usize) >= user_start && (alloc_ptr as usize) < block_end {
                    return block_start as *mut FreeBlock;
                }
            }

            search_addr = search_addr.saturating_sub(1);
            if search_addr < self.heap_start {
                break;
            }
        }

        core::ptr::null_mut()
    }

    unsafe fn insert_free_block(&mut self, block_ptr: usize) {
        let block = &mut *(block_ptr as *mut FreeBlock);
        block.next = core::ptr::null_mut();

        // If free list is empty, just add this block
        if self.free_list.is_null() {
            self.free_list = block_ptr as *mut FreeBlock;
            return;
        }

        // Find the right position in sorted order (by address)
        let mut current = self.free_list;
        let mut prev: *mut FreeBlock = core::ptr::null_mut();

        while !current.is_null() && (current as usize) < block_ptr {
            prev = current;
            current = (*current).next;
        }

        block.next = current;

        if !prev.is_null() {
            (*prev).next = block_ptr as *mut FreeBlock;
        } else {
            self.free_list = block_ptr as *mut FreeBlock;
        }

        // Coalesce with next block if adjacent
        if !current.is_null() && block_ptr + block.header.size == current as usize {
            block.header.size += (*current).header.size;
            block.next = (*current).next;
        }

        // Coalesce with previous block if adjacent
        if !prev.is_null() {
            let prev_block = &mut *(prev as *mut FreeBlock);
            if (prev as usize) + prev_block.header.size == block_ptr {
                prev_block.header.size += block.header.size;
                prev_block.next = block.next;
            }
        }
    }

    unsafe fn grow_heap(&mut self, needed_size: usize) -> Result<(), ()> {
        let current_pages = (self.heap_current_end - self.heap_start) / PAGE_SIZE;
        let needed_pages = (needed_size + PAGE_SIZE - 1) / PAGE_SIZE;

        if current_pages + needed_pages > HEAP_MAX_PAGES {
            return Err(());
        }

        let pml4 = crate::arch::x86_64::paging::current_pml4();

        for i in 0..needed_pages {
            let virt = self.heap_current_end + i * PAGE_SIZE;
            let phys = match crate::memory::allocator::alloc_frame() {
                Some(p) => p,
                None => return Err(()),
            };

            if crate::arch::x86_64::paging::map_page(
                pml4,
                virt as u64,
                phys,
                crate::arch::x86_64::paging::PageTableEntry::PRESENT
                    | crate::arch::x86_64::paging::PageTableEntry::WRITABLE,
            ).is_err() {
                return Err(());
            }
        }

        // Mark the newly mapped region as a free block
        let new_block_ptr = self.heap_current_end as *mut FreeBlock;
        (*new_block_ptr).header.size = needed_pages * PAGE_SIZE;
        (*new_block_ptr).next = self.free_list;
        self.free_list = new_block_ptr;

        self.heap_current_end += needed_pages * PAGE_SIZE;

        Ok(())
    }
}

// Raw pointers are Send/Sync for kernel heap (single-threaded initial use)
unsafe impl Send for LinkedListHeap {}
unsafe impl Sync for LinkedListHeap {}

static KERNEL_HEAP: Spinlock<LinkedListHeap> = Spinlock::new(LinkedListHeap::new(HEAP_START));

struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        KERNEL_HEAP.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        KERNEL_HEAP.lock().dealloc(ptr);
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: KernelAllocator = KernelAllocator;

pub fn init() {
    // Initialize the heap by mapping initial pages
    unsafe {
        let mut heap = KERNEL_HEAP.lock();
        let pml4 = crate::arch::x86_64::paging::current_pml4();

        for i in 0..HEAP_INITIAL_PAGES {
            let virt = HEAP_START + i * PAGE_SIZE;
            let phys = match crate::memory::allocator::alloc_frame() {
                Some(p) => p,
                None => {
                    panic!("Failed to allocate physical frame for heap initialization");
                }
            };

            if crate::arch::x86_64::paging::map_page(
                pml4,
                virt as u64,
                phys,
                crate::arch::x86_64::paging::PageTableEntry::PRESENT
                    | crate::arch::x86_64::paging::PageTableEntry::WRITABLE,
            ).is_err() {
                panic!("Failed to map heap page at {:#x}", virt);
            }
        }

        // Initialize free list as one large block
        let initial_block = HEAP_START as *mut FreeBlock;
        (*initial_block).header.size = HEAP_INITIAL_PAGES * PAGE_SIZE;
        (*initial_block).next = core::ptr::null_mut();
        heap.free_list = initial_block;
        heap.heap_current_end = HEAP_START + HEAP_INITIAL_PAGES * PAGE_SIZE;
    }
}
