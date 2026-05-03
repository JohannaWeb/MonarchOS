#![no_std]

pub mod terminal;

pub mod arch;
pub mod boot_info;
pub mod filesystem;
pub mod io;
pub mod memory;
pub mod process;
pub mod sync;

/// Initialize the kernel
pub fn init() {
    arch::init();
    memory::init();
    process::init();
}
