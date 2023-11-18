#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]

#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;

// app running aspace
// SBI(0x80000000) -> App <- Kernel(0x80200000)
// 0xffff_ffc0_0000_0000
const RUN_START: usize = 0xffff_ffc0_8010_0000;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let apps_start = PLASH_START as *const u8;
    let apps_size = 32; // Dangerous!!! We need to get accurate size of apps.

    println!("Load payload ...");

    let code = unsafe { core::slice::from_raw_parts(apps_start, apps_size) };
    println!("content: {:#x}", bytes_to_usize(&code[..8]));


    let load_size = apps_size;
    let run_code = unsafe {
        core::slice::from_raw_parts_mut(RUN_START as *mut u8 , load_size)
    };
    run_code.copy_from_slice(code);
    println!("run code {:?}, addr = [{:?}]", run_code, run_code.as_ptr());
    
    println!("Load payload ok!");

    println!("execute app...");

    unsafe {
        core::arch::asm!("
            li t2, {run_start}
            jalr t2
            j .", 
            run_start = const RUN_START
        );
    }

}

#[inline]
fn bytes_to_usize(bytes: &[u8]) -> usize {
    usize::from_be_bytes(bytes.try_into().unwrap())
}
