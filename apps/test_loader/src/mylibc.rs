#[cfg(feature = "axstd")]
use axstd::println;

fn strlen(mut s: *const u8) -> usize {
    let mut len = 0;
    let s = unsafe {
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
