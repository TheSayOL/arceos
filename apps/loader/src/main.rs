#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;

struct Header {
    magic: [u8;8],
    app_off: u64,
    app_size: u64,
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let apps_start = PLASH_START as *const u8;

    let header = unsafe { &*(apps_start as *const Header) };
    assert_eq!(header.magic, "UniKernl".as_bytes());

    let apps_size = header.app_size as usize;
    let apps_start = unsafe {apps_start.add(header.app_off as usize)} ; 
    println!("Load payload ...");
    let code = unsafe { core::slice::from_raw_parts(apps_start, apps_size) };
    println!("code = {:x?}", code);
    println!("Load payload ok!");
}

