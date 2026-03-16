pub mod serial;
pub mod console;

pub fn init() {
    serial::init();
    console::init();
}
