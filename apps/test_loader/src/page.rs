use super::config::*;
use axstd::println;

use arceos_api::mem::ax_alloc;
use axstd::vec::Vec;

// App aspace
#[link_section = ".data.app_page_table"]
static mut APP_PT_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP_PT2_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP_PT3_SV39: [u64; 512] = [0; 512];



fn get_one_page() -> usize {
    ax_alloc(core::alloc::Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap()).unwrap().addr().get()
}

fn get_pages(num: usize) -> Vec<usize> {
    let mut v = Vec::new();
    (0..num).for_each(|_| v.push(get_one_page()));
    v
}


pub fn init_app_page_table() {
    
    let v = get_pages(10);
    println!("v = {:x?}", v);
    
    unsafe {
        // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
        APP_PT_SV39[0] = (get_ppn(APP_PT2_SV39.as_ptr() as usize) << 10) | 0x01;
        // 0x00_0000 .. 0x40_0000, 2M
        APP_PT2_SV39[0] = (get_ppn(APP_PT3_SV39.as_ptr() as usize) << 10) | 0x01;
        for i in 0..512 {
            APP_PT3_SV39[0 + i] = ((((CODE_START >> 12) + i) << 10) | 0xcf) as u64;
        }
        // map 0x2200_0000 .. 0x2200_0000 + 32M
        for i in 0..16 {
            APP_PT2_SV39[0x110 + i] = (((0x2_2000 + i) << 10) | 0xcf) as u64;
        }

        // 0x4000_0000..0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
        APP_PT_SV39[1] = (0x80000 << 10) | 0xcf;
        APP_PT_SV39[2] = (0x80000 << 10) | 0xcf;
        // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
        APP_PT_SV39[0x102] = (0x80000 << 10) | 0xcf;
    }
}

pub fn switch_app_aspace() {
    use riscv::register::satp;
    unsafe {
        let page_table_root = APP_PT_SV39.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;
        satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
        riscv::asm::sfence_vma_all();
    }
}

fn get_ppn(va: usize) -> u64 {
    let ret = (va - axconfig::PHYS_VIRT_OFFSET) >> 12;
    ret as u64
}
