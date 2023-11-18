#![feature(asm_const)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#[cfg(feature = "axstd")]
use axstd::println;
#[cfg(feature = "axstd")]
use axstd::os::arceos::api::task::ax_exit;

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

    register_abi(SYS_HELLO, abi_hello as usize);
    register_abi(SYS_PUTCHAR, abi_putchar as usize);
    register_abi(SYS_TERMINATE, abi_terminate as usize);

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

        // println!("content = {:x?}", data);

        apps_start = data_start + data_size;

        let run_code = unsafe { core::slice::from_raw_parts_mut(RUN_START as *mut u8, data_size) };
        run_code.copy_from_slice(data);
        // println!("run code {:x?}; address [{:x?}], executing...", run_code, run_code.as_ptr());
        execute_app();
    }
    println!("Load payload ok!");
}

#[no_mangle]
fn execute_app() {
    let arg0: u8 = b'A';
    // execute app
    unsafe { core::arch::asm!("
        la      a7, {abi_table}
        li      t2, {run_start}
        jalr    t2
        j       .",
        run_start = const RUN_START,
        abi_table = sym ABI_TABLE,
    )}
}


const SYS_HELLO: usize = 1;
const SYS_PUTCHAR: usize = 2;
const SYS_TERMINATE : usize = 3;

static mut ABI_TABLE: [usize; 16] = [0; 16];

fn register_abi(num: usize, handle: usize) {
    unsafe { ABI_TABLE[num] = handle; }
}

fn abi_hello() {
    println!("[ABI:Hello] Hello, Apps!");
}

fn abi_putchar(c: char) {
    println!("[ABI:Print] {c}");
}

fn abi_terminate() {
    println!("[ABI:Terminate] exit...");
    ax_exit(0);
}