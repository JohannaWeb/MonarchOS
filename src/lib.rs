#![no_std]

pub mod boot_info;
pub mod arch;
pub mod memory;
pub mod process;
pub mod filesystem;
pub mod io;
pub mod sync;

/// Initialize the kernel
pub fn init() {
    arch::init();
    memory::init();
    process::init();
}
