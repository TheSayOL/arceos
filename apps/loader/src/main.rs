#![feature(asm_const)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x22000000;
// app running aspace
// SBI(0x80000000) -> App <- Kernel(0x80200000)
// 0xffff_ffc0_0000_0000
const RUN_START: usize = 0xffff_ffc0_8010_0000;

struct Header {
    magic: [u8; 8],
    app_off: u64,
    app_size: u64,
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let mut apps_start = PLASH_START;
    println!("Load payload ...");
    loop {
        let header = unsafe { &*(apps_start as *const Header) };
        if header.magic != "UniKernl".as_bytes() {
            // println!("no more apps.");
            break;
        }
        println!("app start = 0x{:x}", apps_start);
        let data_start = apps_start + header.app_off as usize;
        let data_size = header.app_size as usize;
        let data = unsafe { core::slice::from_raw_parts(data_start as *const u8, data_size) };

        println!("content = {:x?}", data);

        apps_start = data_start + data_size;

        let run_code = unsafe { core::slice::from_raw_parts_mut(RUN_START as *mut u8, data_size) };
        run_code.copy_from_slice(data);
        println!("run code {:x?}; address [{:x?}], executing...", run_code, run_code.as_ptr());
        execute_app();
    }
    println!("Load payload ok!");
}

#[no_mangle]
fn execute_app() {
    // execute app
    unsafe {
        core::arch::asm!("
        li t2, {run_start}
        jalr t2
        ",
        run_start = const RUN_START,
        )
    }
}
