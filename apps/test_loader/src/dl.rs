#[cfg(feature = "axstd")]
use axstd::{println, process::exit};

use super::config::*;
use super::header::Header;
use super::page::{init_app_page_table, switch_app_aspace};
use super::mylibc;

extern crate xmas_elf;

/// read elf data, and do dynamic link
fn from_elf(elf_data: &[u8]) -> usize {
    let mut entry = 0;
    let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
    let elf_header = elf.header;
    let magic = elf_header.pt1.magic;
    assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
    let ph_count = elf_header.pt2.ph_count();

    // load data and code
    for i in 0..ph_count {
        let ph = elf.program_header(i).unwrap();
        if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
            let start_va = ph.virtual_addr() as usize;
            let ph_flags = ph.flags();
            let data = unsafe {
                core::slice::from_raw_parts_mut(start_va as *mut u8, ph.mem_size() as usize)
            };
            data.fill(0);
            let mut index = 0;
            for x in &elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize] {
                data[index] = *x;
                index += 1;
            }
            if ph_flags.is_execute() {
                entry = elf.header.pt2.entry_point() as usize;
            }
        }
    }

    // dynamic link
    for section in elf.section_iter() {
        match section.get_name(&elf) {
            Ok(".rela.dyn") => {
                let mut rela = section.offset() as usize;
                let rela_end = section.size() as usize + rela;
                while rela < rela_end {
                    let p = rela as *const u32;
                    unsafe {
                        let addr = *p as usize;
                        let value = *(p.add(4)) as usize;
                        *(addr as *mut usize) = value as usize + OFFSET;
                    }
                    // size of one entry = 24B in `.rela.dyn`
                    rela += 24;
                }
            }
            Ok(".rela.plt") => {
                let mut rela = section.offset() as usize;
                let rela_end = section.size() as usize + rela;
                let sec = elf.find_section_by_name(".dynsym").unwrap();
                let dynsym = sec.offset() as usize as *const u32;
                while rela < rela_end {
                    let p = rela as *const u32;
                    let addr = unsafe { *p };
                    let dynsym_index = unsafe { *(p.add(3)) as usize };
                    unsafe {
                        match elf.get_dyn_string(*(dynsym.add(dynsym_index * 24 / 4))) {
                            Ok("__libc_start_main") => {
                                *(addr as usize as *mut usize) = __libc_main_start as usize
                            }
                            Ok("puts") => {
                                *(addr as usize as *mut usize) = mylibc::puts as usize
                            }
                            Ok(name) => {
                                println!("name = {}", name);
                            }
                            _ => {}
                        }
                    }
                    rela += 24;
                }
            }
            _ => {}
        }
    }
    entry as usize
}

fn __libc_main_start(c_main: fn() -> i32) {
    let ret = c_main();
    println!("c_main return = {}", ret);
    run_next();
}

struct Runner {
    count: usize,
}

impl Runner {
    fn count(&mut self) -> usize {
        let ret = self.count;
        self.count += 1;
        ret
    }
}

static mut RUNNER: Runner = Runner { count: 0 };

/// start running apps from PLASH
pub fn start_apps() {
    run_next();
}

fn run_next() {
    init_app_page_table();
    switch_app_aspace();
    let i = unsafe { RUNNER.count() };

    let app_start = (PLASH_START + i) as *const u8;
    // my header: magic(UniKernl), appoff, appsize, all u64. just consider it as inode, to get len of ELF file
    let header = unsafe { (app_start as *const Header).as_ref().unwrap() };

    // check magic
    if header.magic != "UniKernl".as_bytes() {
        return;
    }
    let app_off = header.app_off;
    let app_size = header.app_size;

    // read elf
    let data = unsafe { app_start.add(app_off as usize) };
    let data = unsafe { core::slice::from_raw_parts(data, app_size as usize) };
    let entry = from_elf(data);

    // init stack
    let sp = STACK_TOP;

    // execute app
    unsafe {
        core::arch::asm!("
        mv      sp,t2
        jalr    t1
        ",
        in("t1")entry,
        in("t2")sp,
        )
    };
    // exit, or it will return to `__libc_start_main`
    exit(0);
}
