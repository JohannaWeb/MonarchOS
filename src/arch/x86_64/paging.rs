/// Virtual memory paging initialization

// Compile-time checks
const _: () = assert!(core::mem::size_of::<PageTableEntry>() == 8);
const _: () = assert!(core::mem::size_of::<PageTable>() == 4096);
const _: () = assert!(core::mem::align_of::<PageTable>() == 4096);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const NO_CACHE: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE_PAGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;
    pub const NO_EXECUTE: u64 = 1 << 63;
    pub const PHYS_ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    pub const fn missing() -> Self {
        Self(0)
    }

    pub fn is_present(self) -> bool {
        self.0 & Self::PRESENT != 0
    }

    pub fn is_huge(self) -> bool {
        self.0 & Self::HUGE_PAGE != 0
    }

    pub fn phys_addr(self) -> u64 {
        self.0 & Self::PHYS_ADDR_MASK
    }

    pub fn new(phys: u64, flags: u64) -> Self {
        Self(phys | flags)
    }
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

impl PageTable {
    fn new() -> Self {
        PageTable {
            entries: [PageTableEntry::missing(); 512],
        }
    }
}

/// Translate a physical address to virtual using HHDM offset
pub fn phys_to_virt(phys: u64) -> *mut u8 {
    (phys + crate::boot_info::get().hhdm_offset) as *mut u8
}

/// Read CR3 to get physical address of active PML4
pub fn current_pml4() -> *mut PageTable {
    let cr3: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, nomem));
    }
    phys_to_virt(cr3 & PageTableEntry::PHYS_ADDR_MASK) as *mut PageTable
}

#[derive(Debug)]
pub enum MapError {
    AlreadyMapped,
    NotMapped,
    OutOfFrames,
}

/// Map virt → phys in the given PML4 (4KB pages)
pub unsafe fn map_page(
    pml4: *mut PageTable,
    virt: u64,
    phys: u64,
    flags: u64,
) -> Result<(), MapError> {
    let pml4_idx = (virt >> 39) as usize & 0x1FF;
    let pdpt_idx = (virt >> 30) as usize & 0x1FF;
    let pd_idx = (virt >> 21) as usize & 0x1FF;
    let pt_idx = (virt >> 12) as usize & 0x1FF;

    // Walk/create PML4 → PDPT
    let mut pml4_entry = (*pml4).entries[pml4_idx];
    let pdpt = if pml4_entry.is_present() {
        phys_to_virt(pml4_entry.phys_addr()) as *mut PageTable
    } else {
        let new_frame = crate::memory::allocator::alloc_frame()
            .ok_or(MapError::OutOfFrames)?;
        let new_pdpt = phys_to_virt(new_frame) as *mut PageTable;
        core::ptr::write(new_pdpt, PageTable::new());
        pml4_entry = PageTableEntry::new(new_frame, PageTableEntry::PRESENT | PageTableEntry::WRITABLE);
        (*pml4).entries[pml4_idx] = pml4_entry;
        new_pdpt
    };

    // Walk/create PDPT → PD
    let mut pdpt_entry = (*pdpt).entries[pdpt_idx];
    let pd = if pdpt_entry.is_present() {
        phys_to_virt(pdpt_entry.phys_addr()) as *mut PageTable
    } else {
        let new_frame = crate::memory::allocator::alloc_frame()
            .ok_or(MapError::OutOfFrames)?;
        let new_pd = phys_to_virt(new_frame) as *mut PageTable;
        core::ptr::write(new_pd, PageTable::new());
        pdpt_entry = PageTableEntry::new(new_frame, PageTableEntry::PRESENT | PageTableEntry::WRITABLE);
        (*pdpt).entries[pdpt_idx] = pdpt_entry;
        new_pd
    };

    // Walk/create PD → PT
    let mut pd_entry = (*pd).entries[pd_idx];
    let pt = if pd_entry.is_present() {
        phys_to_virt(pd_entry.phys_addr()) as *mut PageTable
    } else {
        let new_frame = crate::memory::allocator::alloc_frame()
            .ok_or(MapError::OutOfFrames)?;
        let new_pt = phys_to_virt(new_frame) as *mut PageTable;
        core::ptr::write(new_pt, PageTable::new());
        pd_entry = PageTableEntry::new(new_frame, PageTableEntry::PRESENT | PageTableEntry::WRITABLE);
        (*pd).entries[pd_idx] = pd_entry;
        new_pt
    };

    // Map page in PT
    let pt_entry = (*pt).entries[pt_idx];
    if pt_entry.is_present() {
        return Err(MapError::AlreadyMapped);
    }

    (*pt).entries[pt_idx] = PageTableEntry::new(phys, flags);
    Ok(())
}

/// Unmap virt in the given PML4; returns mapped physical address
pub unsafe fn unmap_page(pml4: *mut PageTable, virt: u64) -> Result<u64, MapError> {
    let pml4_idx = (virt >> 39) as usize & 0x1FF;
    let pdpt_idx = (virt >> 30) as usize & 0x1FF;
    let pd_idx = (virt >> 21) as usize & 0x1FF;
    let pt_idx = (virt >> 12) as usize & 0x1FF;

    // Walk PML4 → PDPT
    let pml4_entry = (*pml4).entries[pml4_idx];
    if !pml4_entry.is_present() {
        return Err(MapError::NotMapped);
    }
    let pdpt = phys_to_virt(pml4_entry.phys_addr()) as *mut PageTable;

    // Walk PDPT → PD
    let pdpt_entry = (*pdpt).entries[pdpt_idx];
    if !pdpt_entry.is_present() {
        return Err(MapError::NotMapped);
    }
    let pd = phys_to_virt(pdpt_entry.phys_addr()) as *mut PageTable;

    // Walk PD → PT
    let pd_entry = (*pd).entries[pd_idx];
    if !pd_entry.is_present() {
        return Err(MapError::NotMapped);
    }
    let pt = phys_to_virt(pd_entry.phys_addr()) as *mut PageTable;

    // Unmap page in PT
    let pt_entry = (*pt).entries[pt_idx];
    if !pt_entry.is_present() {
        return Err(MapError::NotMapped);
    }

    let phys = pt_entry.phys_addr();
    (*pt).entries[pt_idx] = PageTableEntry::missing();
    Ok(phys)
}

pub fn init() {
    let boot = crate::boot_info::get();
    let pml4 = current_pml4();

    // Verify PML4 entry for kernel higher-half
    let pml4_idx = (boot.kernel_virt_base >> 39) as usize & 0x1FF;
    let pml4_entry = unsafe { (*pml4).entries[pml4_idx] };
    assert!(
        pml4_entry.is_present(),
        "paging: PML4[{}] not present",
        pml4_idx
    );

    // Walk one level deeper
    let pdpt = phys_to_virt(pml4_entry.phys_addr()) as *mut PageTable;
    let pdpt_idx = (boot.kernel_virt_base >> 30) as usize & 0x1FF;
    let pdpt_entry = unsafe { (*pdpt).entries[pdpt_idx] };
    assert!(
        pdpt_entry.is_present(),
        "paging: PDPT[{}] not present",
        pdpt_idx
    );

    // Confirm HHDM is accessible with read_volatile
    let hhdm_test = boot.hhdm_offset as *const u8;
    unsafe {
        core::ptr::read_volatile(hhdm_test);
    }
}
