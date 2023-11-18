#![feature(asm_const)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#[cfg(feature = "axstd")]
use axstd::os::arceos::api::task::ax_exit;
#[cfg(feature = "axstd")]
use axstd::println;

const SYS_HELLO: usize = 1;
const SYS_PUTCHAR: usize = 2;
const SYS_TERMINATE: usize = 3;

static mut ABI_TABLE: [usize; 16] = [0; 16];

fn register_abi(num: usize, handle: usize) {
    unsafe {
        ABI_TABLE[num] = handle;
    }
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

const PLASH_START: usize = 0x22000000;
// app running aspace
// SBI(0x80000000) -> App <- Kernel(0x80200000)
// 0xffff_ffc0_0000_0000
const RUN_START: usize = 0x4010_0000;

//
// App aspace
//

#[link_section = ".data.app_page_table"]
static mut APP_PT_SV39: [u64; 512] = [0; 512];

unsafe fn init_app_page_table() {
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[2] = (0x80000 << 10) | 0xef;
    // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0x102] = (0x80000 << 10) | 0xef;

    // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0] = (0x00000 << 10) | 0xef;

    // For App aspace!
    // 0x4000_0000..0x8000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[1] = (0x80000 << 10) | 0xef;
}

unsafe fn switch_app_aspace() {
    use riscv::register::satp;
    let page_table_root = APP_PT_SV39.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;
    satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
    riscv::asm::sfence_vma_all();
}

// #[repr(C)]
// struct Header {
//     magic: [u8; 8],
//     app_off: u64,
//     app_size: u64,
// }

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // switch aspace from kernel to app
    unsafe {
        init_app_page_table();
    }
    unsafe {
        switch_app_aspace();
    }

    register_abi(SYS_HELLO, abi_hello as usize);
    register_abi(SYS_PUTCHAR, abi_putchar as usize);
    register_abi(SYS_TERMINATE, abi_terminate as usize);

    let mut apps_start = PLASH_START;
    println!("Load payload ...");
    loop {
        let magic = unsafe { core::slice::from_raw_parts(apps_start as *const u8, 8)   };
        // println!("magic = {:x?}", core::str::from_utf8(magic).unwrap());
        if magic != "UniKernl".as_bytes() {
            break;
        }
        let app_off = unsafe {core::slice::from_raw_parts((apps_start+8) as *const u8,8)};
        let app_off = unsafe { u64::from_le_bytes(app_off.try_into().unwrap()) };
        let app_size = unsafe {core::slice::from_raw_parts((apps_start+16) as *const u8,8)};
        let app_size = unsafe { u64::from_le_bytes(app_size.try_into().unwrap()) };
        let data_start = apps_start + app_off as usize;
        let data_size = app_size as usize;
        let data = unsafe { core::slice::from_raw_parts(data_start as *const u8, data_size) };
        // println!("data = {:?}", data);

        let run_code = unsafe { core::slice::from_raw_parts_mut(RUN_START as *mut u8, data_size) };

        // clean  
        run_code.fill(0);
        
        run_code.copy_from_slice(data);
        // println!(
        //     "data_start, data_sizse = 0x{:x}, 0x{:x}",
        //     data_start, data_size
        // );

        apps_start += (app_off + app_size) as usize;

        // execute app
        unsafe {
            core::arch::asm!("
            la      a7, {abi_table}
            li      t2, {run_start}
            jalr    t2
            ",
                run_start = const RUN_START,
                abi_table = sym ABI_TABLE,
            )
        };
    }
    println!("Load payload ok!");
}

#[inline]
fn bytes_to_usize(bytes: &[u8]) -> usize {
    usize::from_be_bytes(bytes.try_into().unwrap())
}
