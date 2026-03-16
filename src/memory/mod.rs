pub mod allocator;
pub mod manager;

pub fn init() {
    allocator::init();
    manager::init();
}
