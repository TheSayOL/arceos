#![feature(asm_const)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#[cfg(feature = "axstd")]
use axstd::println;

const PLASH_START: usize = 0x2200_0000;
// app running aspace
// SBI(0x80000000) -> App <- Kernel(0x80200000)
// 0xffff_ffc0_0000_0000
// const RUN_START: usize = 0x8050_0000;
const CODE_START: usize = 0x8010_0000;
const DATA_START: usize = 0x8510_0000;
const ALL_BYTES: usize = 750584;
static mut MUSL: [u8; ALL_BYTES] = [0u8; ALL_BYTES];

#[repr(C)]
#[derive(Debug)]
struct Header {
    magic: [u8; 8],
    app_off: u64,
    app_size: u64,
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    unsafe {
        init_app_page_table();
        switch_app_aspace();
    }

    let mut apps_start = (PLASH_START) as *const u8;

    let header = unsafe { (apps_start as *const Header).as_ref().unwrap() };

    // check magic
    assert_eq!(header.magic, "UniKernl".as_bytes());

    // read data
    let app_off = header.app_off;
    // let app_size = ALL_BYTES;
    let app_size = header.app_size;

    println!("{:?}", header);

    println!("Load payload ...");
    unsafe { apps_start = apps_start.add(app_off as usize) };
    let data = unsafe { core::slice::from_raw_parts(apps_start, app_size as usize) };
    println!("copying...");
    // unsafe {MUSL.copy_from_slice(data);}
    println!("copy ok");

    // from_elf(&MUSL);
    let entry = from_elf(data);

    // write data to RUN_START
    // let run_code = unsafe { core::slice::from_raw_parts_mut(CODE_START as *mut u8, data_size) };
    // run_code.copy_from_slice(data);

    // execute app
    let run_start = CODE_START + entry;
    unsafe {
        core::arch::asm!(
            "
        nop
        nop
        nop
        nop
        nop
        nop
        nop
        jalr    t2
        ",
        in("t2")run_start
        )
    };
    println!("Load payload ok!");
}

extern crate xmas_elf;
pub fn from_elf(elf_data: &[u8]) -> usize {
    let mut en = 0;
    println!("data addr = {:x}", elf_data.as_ptr() as usize);
    // map program headers of elf, with U flag
    let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
    println!("read ok");
    let elf_header = elf.header;
    println!("header = {:#?}", elf_header);
    let magic = elf_header.pt1.magic;
    assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
    let ph_count = elf_header.pt2.ph_count();
    println!("ph count = {}", ph_count);
    for i in 0..ph_count {
        let ph = elf.program_header(i).unwrap();
        if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
            println!("can load------------------------");
            println!("ph = {:#?}", &ph);
            let start_va = ph.virtual_addr() as usize;
            let end_va = (ph.virtual_addr() + ph.mem_size()) as usize;
            let ph_flags = ph.flags();
            println!("start va {:x} end va {:x}", start_va, end_va);
            println!("flags = {:?}", ph_flags);
            if ph_flags.is_read() {
                println!("read");
            }
            if ph_flags.is_write() {
                println!("write");
                unsafe {
                    let memsize = end_va - start_va;
                    println!("va need = {}", memsize);
                    println!("file need = {}", ph.file_size());
                    let data = { DATA_START as *mut u8 };
                    let data = core::slice::from_raw_parts_mut(data, memsize);
                    data.fill(0);
                    let mut i = 0;
                    for x in
                        &elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]
                    {
                        data[i] = *x;
                        i += 1;
                    }
                    // data.copy_from_slice(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]);
                }
            }
            if ph_flags.is_execute() {
                println!("exe");
                unsafe {
                    let memsize = end_va - start_va;
                    println!("va need = {}", memsize);
                    println!("file need = {}", ph.file_size());
                    let code = { CODE_START as *mut u8 };
                    let code = core::slice::from_raw_parts_mut(code, memsize);
                    code.fill(0);
                    let start = elf.header.pt2.header_size() as u64
                        + (ph_count * elf.header.pt2.ph_entry_size()) as u64
                        + ph.offset();
                    println!("start = 0x{:x}, {}", start, start);
                    println!(
                        "code = {:x?}",
                        &elf_data[start as usize..start as usize + 30]
                    );
                    code.copy_from_slice(
                        &elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize],
                    );
                    println!("ph offset = {}, filesize = {}", ph.offset(), ph.file_size());
                    println!("code = {:x?}", &code[..200]);
                    println!("code = {:x?}", &elf_data[..200]);

                    en = elf.header.pt2.entry_point() as usize;
                    println!("entry = {:x}", en);
                }
            }

            println!("ph.offset = {:x}", ph.offset());
            println!("ph.offset = {:x}", ph.file_size());
        }
    }
    en
}

// App aspace
#[link_section = ".data.app_page_table"]
static mut APP_PT_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP_PT2_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP_PT3_SV39: [u64; 512] = [0; 512];
#[link_section = ".data.app_page_table"]
static mut APP_PT3_2_SV39: [u64; 512] = [0; 512];

unsafe fn init_app_page_table() {
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[2] = (0x80000 << 10) | 0xcf;
    // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0x102] = (0x80000 << 10) | 0xcf;

    // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
    APP_PT_SV39[0] = (0x0 << 10) | 0xcf;
    // APP_PT_SV39[0] = (get_ppn(APP_PT2_SV39.as_ptr() as usize) << 10) | 0x01;
    // // 0x0 .. 0x40_0000, 2M
    // APP_PT2_SV39[0] = (get_ppn(APP_PT3_SV39.as_ptr() as usize) << 10) | 0x01;
    // // 0x40_0000 .. 0x80_0000, 2M
    // APP_PT2_SV39[1] = (get_ppn(APP_PT3_2_SV39.as_ptr() as usize) << 10) | 0x01;
    // for i in 0..512 {
    //     APP_PT3_SV39[0 + i] = (((0x8_5000 + i) << 10) | 0xcf) as u64;
    //     APP_PT3_2_SV39[0 + i] = (((0x8_6000 + i) << 10) | 0xcf) as u64;
    // }
    // // map 0x2200_0000 ..  + 32M
    // for i in 0..16 {
    //     APP_PT2_SV39[0x110 + i] = (((0x2_2000 + i) << 10) | 0xcf) as u64;
    // }

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

fn get_ppn(va: usize) -> u64 {
    let ret = (va - axconfig::PHYS_VIRT_OFFSET) >> 12;
    ret as u64
}

/*
0x80100000:     0x464c457f      0x00010102      0x00000000      0x00000000
0x80100010:     0x00f30003      0x00000001      0x0000048e      0x00000000
0x80100020:     0x00000040      0x00000000
*/
