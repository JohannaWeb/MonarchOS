#![no_std]
#![no_main]
#![allow(internal_features)]
#![feature(lang_items)]

use core::panic::PanicInfo;
use limine::request::{FramebufferRequest, MemoryMapRequest, RsdpRequest};

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
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Get framebuffer for early output
    if let Some(response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(fb) = response.framebuffers().next() {
            // Simple early output
            let addr = fb.addr() as *mut u32;
            let width = fb.width() as usize;
            let height = fb.height() as usize;
            let pitch = fb.pitch() as usize;

            // Clear framebuffer to black
            for y in 0..height {
                for x in 0..width {
                    let pixel_offset = y * (pitch / 4) + x;
                    unsafe {
                        addr.add(pixel_offset).write_volatile(0xFF_000000);
                    }
                }
            }
        }
    }

    // Get memory map
    if let Some(response) = MEMORY_MAP_REQUEST.get_response() {
        let _total_memory = response.entries().iter().fold(0u64, |acc, entry| {
            acc + entry.length
        });
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
