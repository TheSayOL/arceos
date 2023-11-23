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

const PLASH_START: usize = 0x2200_0000;
// app running aspace
// SBI(0x80000000) -> App <- Kernel(0x80200000)
// 0xffff_ffc0_0000_0000
const RUN_START: usize = 0x4010_0000;

//
// App aspace
//
#[link_section = ".data.app_page_table"]
static mut APP_PT_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP_PT2_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP_PT3_SV39: [u64; 512] = [0; 512];

#[link_section = ".data.app_page_table"]
static mut APP2_PT_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP2_PT2_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP2_PT3_SV39: [u64; 512] = [0; 512];

/// 2  app
/// first 0x4010_0000 -> 0x8010_0000
/// secnd 0x4010_0000 -> 0x8010_1000
/// va: 0x
unsafe fn init_app_page_table() {
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[2] = (0x80000 << 10) | 0xcf;
    APP2_PT_SV39[2] = (0x80000 << 10) | 0xcf;
    // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0x102] = (0x80000 << 10) | 0xcf;
    APP2_PT_SV39[0x102] = (0x80000 << 10) | 0xcf;

    // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0] = (0x00000 << 10) | 0xcf;
    APP2_PT_SV39[0] = (0x00000 << 10) | 0xcf;

    // For App aspace!
    // 0x4000_0000..0x8000_0000, VRWX_GAD, 1G block

    // // one level page table
    // APP_PT_SV39[1] = (0x80000 << 10) | 0xcf;
    // // two level page table
    // APP_PT_SV39[1] = (get_ppn(APP_PT2_SV39.as_ptr() as usize) << 10) | 0x01;
    // APP_PT2_SV39[0] = (0x80000 << 10) | 0xcf;

    // three level page table
    // app 1: 0x4010_0000 -> 0x8010_0000
    APP_PT_SV39[1] = (get_ppn(APP_PT2_SV39.as_ptr() as usize) << 10) | 0x01;
    APP_PT2_SV39[0] = (get_ppn(APP_PT3_SV39.as_ptr() as usize) << 10) | 0x01;
    // map 10 pages: 0x80100 ~ 0x80109
    for i in 0..10 {
        APP_PT3_SV39[0x100 + i] = (((0x80100 + i) << 10) | 0xcf) as u64;
    }

    // app 2: 0x4010_0000 -> 0x8010_a000
    APP2_PT_SV39[1] = (get_ppn(APP2_PT2_SV39.as_ptr() as usize) << 10) | 0x01;
    APP2_PT2_SV39[0] = (get_ppn(APP2_PT3_SV39.as_ptr() as usize) << 10) | 0x01;
    // map 10 pages: 0x8010a ~ 0x80114
    for i in 0..10 {
        APP2_PT3_SV39[0x100 + i] = (((0x8010a + i) << 10) | 0xcf) as u64;
    }
}

fn get_ppn(va: usize) -> u64 {
    let ret = (va - axconfig::PHYS_VIRT_OFFSET) >> 12;
    ret as u64
}

unsafe fn switch_app_aspace(index: usize) {
    use riscv::register::satp;
    let mut page_table_root = APP_PT_SV39.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;
    if index == 1 {
        page_table_root = APP2_PT_SV39.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;
    }
    satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
    riscv::asm::sfence_vma_all();
}

#[repr(C)]
struct Header {
    magic: [u8; 8],
    app_off: u64,
    app_size: u64,
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    unsafe {
        init_app_page_table();
    }

    register_abi(SYS_HELLO, abi_hello as usize);
    register_abi(SYS_PUTCHAR, abi_putchar as usize);
    register_abi(SYS_TERMINATE, abi_terminate as usize);

    let mut apps_start = PLASH_START;

    println!("Load payload ...");
    // loop for each app:
    for i in 0..2 {

        // switch space
        unsafe {
            switch_app_aspace(i);
        }
        let header = unsafe { (apps_start as *const Header).as_ref().unwrap() };

        // check magic
        assert_eq!(header.magic, "UniKernl".as_bytes());

        // read data
        let app_off = header.app_off;
        let app_size = header.app_size;
        let data_start = apps_start + app_off as usize;
        let data_size = app_size as usize;
        let data = unsafe { core::slice::from_raw_parts(data_start as *const u8, data_size) };
        apps_start += (app_off + app_size) as usize;

        // write data to RUN_START
        let run_code = unsafe { core::slice::from_raw_parts_mut(RUN_START as *mut u8, data_size) };
        run_code.copy_from_slice(data);

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
