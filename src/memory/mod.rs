pub mod allocator;
pub mod manager;
pub mod heap;

pub fn init() {
    allocator::init();
    manager::init();
    heap::init();
}
