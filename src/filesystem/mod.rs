pub mod vfs;
pub mod ext2;

pub fn init() {
    vfs::init();
}
