#![no_std]
#![no_main]
#![allow(internal_features)]
#![feature(lang_items)]

extern crate alloc;

use core::panic::PanicInfo;
use limine::request::{
    ExecutableAddressRequest, FramebufferRequest, HhdmRequest, MemoryMapRequest, RequestsEndMarker,
    RequestsStartMarker, RsdpRequest,
};
use limine::BaseRevision;

// Provide compiler intrinsics for bare-metal
#[no_mangle]
pub extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe {
        core::ptr::copy_nonoverlapping(src, dest, n);
    }
    dest
}

#[no_mangle]
pub extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe {
        core::ptr::copy(src, dest, n);
    }
    dest
}

#[no_mangle]
pub extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    unsafe {
        core::ptr::write_bytes(s, c as u8, n);
    }
    s
}

#[no_mangle]
pub extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe {
        for i in 0..n {
            let a = *s1.add(i);
            let b = *s2.add(i);
            if a != b {
                return (a as i32) - (b as i32);
            }
        }
    }
    0
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

// Limine requests
#[used]
#[link_section = ".limine_requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".limine_requests"]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[link_section = ".limine_requests"]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[used]
#[link_section = ".limine_requests"]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".limine_requests"]
static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

#[used]
#[link_section = ".limine_requests_start"]
static _REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[link_section = ".limine_requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".limine_requests_end"]
static _REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Set up boot info before initializing kernel
    let hhdm_offset = HHDM_REQUEST.get_response().expect("no HHDM").offset();
    let exec = EXECUTABLE_ADDRESS_REQUEST
        .get_response()
        .expect("no ExecutableAddress");
    let mmap = MEMORY_MAP_REQUEST
        .get_response()
        .expect("no MemoryMap")
        .entries();
    unsafe {
        monarch::boot_info::set(monarch::boot_info::BootInfo {
            hhdm_offset,
            kernel_phys_base: exec.physical_base(),
            kernel_virt_base: exec.virtual_base(),
            memory_map: mmap,
        });
    }

    // Get framebuffer for early output and initialize terminal first
    let mut terminal_opt = None;
    if let Some(response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(fb) = response.framebuffers().next() {
            let addr = fb.addr() as *mut u32;
            let width = fb.width() as usize;
            let height = fb.height() as usize;
            let pitch = fb.pitch() as usize;

            let mut terminal = monarch::terminal::Terminal::new(addr, width, height, pitch, 2);
            terminal.clear();
            terminal.print("Terminal initialized.\n");
            terminal_opt = Some(terminal);
        }
    }

    if let Some(term) = &mut terminal_opt {
        term.print("Initializing CPU (GDT, IDT, exception handling)...\n");
    }

    // Initialize CPU (GDT, IDT, exception handling)
    monarch::init();

    if let Some(term) = &mut terminal_opt {
        term.print("Testing heap allocator...\n");
    }

    // Test heap allocator
    {
        use alloc::boxed::Box;
        use alloc::vec::Vec;

        // Test Box allocation
        let _boxed = Box::new(42u32);

        // Test Vec allocation
        let mut vec = Vec::new();
        for i in 0..1000u64 {
            vec.push(i);
        }

        // Just ensure we can read from the vec
        let _first = vec[0];
    }

    if let Some(mut term) = terminal_opt {
        term.print("Starting shell...\n");
        term.run_shell();
    }

    // Get memory map
    if let Some(response) = MEMORY_MAP_REQUEST.get_response() {
        let _total_memory = response
            .entries()
            .iter()
            .fold(0u64, |acc, entry| acc + entry.length);
    }

    // Halt
    halt()
}

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt()
}
