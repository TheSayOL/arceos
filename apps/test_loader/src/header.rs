/// consider it as an inode, to get len of ELF file
#[repr(C)]
#[derive(Debug)]
pub struct Header {
    pub magic: [u8; 8],
    pub app_off: u64,
    pub app_size: u64,
}