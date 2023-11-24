#![feature(asm_const)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#[cfg(feature = "axstd")]
use axstd::println;


const PLASH_START: usize = 0x2200_0000;
// app running aspace
// SBI(0x80000000) -> App <- Kernel(0x80200000)
// 0xffff_ffc0_0000_0000
const RUN_START: usize = 0x8050_0000;


#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let apps_start = PLASH_START as *const u8;

    println!("Load payload ...");
    const ALL_BYTES: usize = 750584;

    let data = unsafe { core::slice::from_raw_parts(apps_start, ALL_BYTES) };
    from_elf(data);


    // read data
    // let app_off = header.app_off;
    // let app_size = header.app_size;
    // let data_start = apps_start + app_off as usize;
    // let data_size = app_size as usize;
    // let data = unsafe { core::slice::from_raw_parts(data_start as *const u8, data_size) };
    // apps_start += (app_off + app_size) as usize;

    // // write data to RUN_START
    // let run_code = unsafe { core::slice::from_raw_parts_mut(RUN_START as *mut u8, data_size) };
    // run_code.copy_from_slice(data);

    // // execute app
    // unsafe {
    //     core::arch::asm!("
    //                 li      t2, {run_start}
    //                 jalr    t2
    //                 ",
    //         run_start = const RUN_START,
    //     )
    // };
    println!("Load payload ok!");
}

#[inline]
fn bytes_to_usize(bytes: &[u8]) -> usize {
    usize::from_be_bytes(bytes.try_into().unwrap())
}

extern crate xmas_elf;
pub fn from_elf(elf_data: &[u8]) {
    // map program headers of elf, with U flag
    let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
    let elf_header = elf.header;
    // println!("header = {:#?}", elf_header);
    let magic = elf_header.pt1.magic;
    assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
    let ph_count = elf_header.pt2.ph_count();
    for i in 0..ph_count {
        let ph = elf.program_header(i).unwrap();
        if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
            let start_va = ph.virtual_addr() as usize;
            // let end_va = ph.virtual_addr() + ph.mem_size() as usize;
            let ph_flags = ph.flags();
            // error!("start va {:x} end va {:x}", start_va.0, end_va.0);
            if ph_flags.is_read() {}
            if ph_flags.is_write() {}
            if ph_flags.is_execute() {}
            println!("ph.offset = {:x}", ph.offset());
            println!("ph.offset = {:x}", ph.file_size());
        }
    }
}





// App aspace
#[link_section = ".data.app_page_table"]
static mut APP_PT_SV39: [u64; 512] = [0; 512];

unsafe fn init_app_page_table() {
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[2] = (0x80000 << 10) | 0xcf;
    // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0x102] = (0x80000 << 10) | 0xcf;

    // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0] = (0x00000 << 10) | 0xcf;

    // For App aspace!
    // 0x4000_0000..0x8000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[1] = (0x80000 << 10) | 0xcf;
}



unsafe fn switch_app_aspace() {
    use riscv::register::satp;
    let mut page_table_root = APP_PT_SV39.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;
    satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
    riscv::asm::sfence_vma_all();
}