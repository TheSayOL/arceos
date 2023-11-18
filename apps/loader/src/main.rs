#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;

struct Header {
    magic: [u8; 8],
    app_off: u64,
    app_size: u64,
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let mut apps_start = PLASH_START;
    loop {
        let header = unsafe { &*(apps_start as *const Header) };
        if header.magic != "UniKernl".as_bytes() {
            println!("no more apps.");
            break;
        }
        println!("found app in addr: {:x}", apps_start);
        let data_start = apps_start + header.app_off as usize;
        let data_size = header.app_size as usize;
        let data = unsafe {
            core::slice::from_raw_parts(data_start as *const u8, data_size)
        };

        println!("Load payload ...");
        println!("app data = {:x?}", data);
        println!("Load payload ok!");

        apps_start = data_start + data_size;
    }
}
