use core::ffi::c_void;

#[cfg(feature = "axstd")]
use axstd::println;

pub fn strlen(mut s: *const u8) -> usize {
    let mut len = 0;
    unsafe {
        while *s != 0 {
            len += 1;
            s = s.add(1);
        }
    };
    len
}

pub fn puts(s: *const u8) {
    let len = strlen(s);
    let s = unsafe {
        let s = core::slice::from_raw_parts(s, len);
        core::str::from_utf8(s).unwrap()
    };
    println!("{}",s);
}

pub fn __libc_main_start(c_main: fn() -> i32) {
    println!("__libc_main_start: c_main = 0x{:x}", c_main as usize);
    let ret = c_main();
    println!("__libc_main_start: c_main return = {}", ret);
    // exit(ret);
}


pub fn malloc(size: usize) -> *mut c_void {

    0 as *mut _
}