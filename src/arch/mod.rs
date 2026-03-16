#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

/// Initialize architecture-specific components
pub fn init() {
    #[cfg(target_arch = "x86_64")]
    x86_64::init();
}

/// Architecture-specific halt
pub fn halt() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
