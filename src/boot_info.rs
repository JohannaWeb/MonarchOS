use limine::memory_map;

pub struct BootInfo {
    pub hhdm_offset: u64,
    pub kernel_phys_base: u64,
    pub kernel_virt_base: u64,
    pub memory_map: &'static [&'static memory_map::Entry],
}

static mut BOOT_INFO: Option<BootInfo> = None;

/// Safety: must be called exactly once, before monarch::init()
pub unsafe fn set(info: BootInfo) {
    BOOT_INFO = Some(info);
}

/// Panics if called before set()
pub fn get() -> &'static BootInfo {
    unsafe { BOOT_INFO.as_ref().expect("boot_info not initialized") }
}
