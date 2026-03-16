pub mod gdt;
pub mod idt;
pub mod paging;

pub fn init() {
    gdt::init();
    idt::init();
    paging::init();
}
