use crate::config::*;
use crate::{pagetable_map_mib, pagetable_unmap_mib, Vec};

/// consider PLASH as a 32M disk,
/// every ELF file in it is with a header, like inode,
/// to specify the size of this file.
#[repr(C)]
#[derive(Debug)]
struct Header {
    pub magic: [u8; 8],
    pub app_off: u64,
    pub app_size: u64,
}

/// open ELF files in PLASH,
/// return a `vec`, each element is a file data
pub fn open_plash() -> Vec<Vec<u8>> {
    let mut datas = Vec::new();
    let mut file_addr = PLASH_START;

    // temporarily map plash
    pagetable_map_mib(file_addr, 32);
        use axstd::println;

    loop {
        println!("1");

        // get file header and check magic
        let file_header = unsafe { (file_addr as *const Header).as_ref().unwrap() };
        if file_header.magic != "UniKernl".as_bytes() {
            break;
        }

        // get data
        let data_addr = file_addr + file_header.app_off as usize;
        let mut data = Vec::new();
        let slice = unsafe {
            core::slice::from_raw_parts(data_addr as *const u8, file_header.app_size as usize)
        };
        for s in slice {
            data.push(*s);
        }
        datas.push(data);

        // loop
        file_addr += (file_header.app_off + file_header.app_size) as usize;
    }

    // unmap
    pagetable_unmap_mib(PLASH_START, 32);
    datas
}
