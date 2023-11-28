//! two parts:
//! - read elf file: inspired by rCore
//! - relocate .got
//!
//! FIXME: too many raw pointers

use axstd::vec::Vec;

use super::config::*;
use super::mylibc;

extern crate xmas_elf;

pub struct Segment {
    pub start_va: usize,
    pub len: usize,
    pub data: Vec<u8>,
}

impl Segment {
    fn new(start_va: usize, len: usize) -> Self {
        Self {
            start_va,
            len,
            data: Vec::new(),
        }
    }
}

pub struct ElfData {
    data: Vec<Segment>,
    entry: usize,
}

impl ElfData {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            entry: 0,
        }
    }
    pub fn entry(&self) -> usize {
        self.entry
    }
    pub fn data(&self) -> &Vec<Segment> {
        &self.data
    }
    fn set_entry(&mut self, entry: usize) {
        self.entry = entry + APP_START_VA;
    }
    fn add_segment(&mut self, start_va: usize, len: usize, data: &[u8]) {
        assert!(len >= data.len());
        let mut s = Segment::new(start_va + APP_START_VA, len);
        // len maybe bigger then data.len()
        for i in 0..len {
            if i < data.len() {
                s.data.push(data[i]);
            } else {
                s.data.push(0)
            }
        }
        self.data.push(s);
    }
    fn read<T>(&self, vaddr: usize) -> *const T {
        let vaddr = vaddr + APP_START_VA;
        for s in self.data.iter() {
            // println!("sva {:x}, va {:X}", s.start_va, vaddr);
            if s.start_va <= vaddr && vaddr <= s.start_va + s.len {
                let p = &s.data[vaddr - s.start_va];
                let p = p as *const _ as *const T;
                return p;
            }
        }
        0 as *const _
    }
    fn write<T: Copy>(&mut self, vaddr: usize, value: T) {
        unsafe {
            *(self.read::<T>(vaddr) as *mut T) = value;
        }
    }
}

/// read elf data, and do dynamic link
pub fn from_elf(elf_data: &[u8]) -> ElfData {
    let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
    let elf_header = elf.header;
    let magic = elf_header.pt1.magic;
    assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
    let ph_count = elf_header.pt2.ph_count();

    let mut ret_data = ElfData::new();
    ret_data.set_entry(elf.header.pt2.entry_point() as usize);

    // load LOAD segments
    for i in 0..ph_count {
        let ph = elf.program_header(i).unwrap();
        if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
            let data = &elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize];
            ret_data.add_segment(ph.virtual_addr() as usize, ph.mem_size() as usize, data);
        }
    }

    // dynamic link
    for section in elf.section_iter() {
        match section.get_name(&elf) {
            Ok(".rela.dyn") => {
                let mut rela_addr = section.address() as usize;
                let rela_end = section.size() as usize + rela_addr;
                // rela's entry is 24B = 6 * 4B, and entry[0] is offset, entry[4] is value
                while rela_addr < rela_end {
                    unsafe {
                        let addr = ret_data.read::<usize>(rela_addr);
                        let value = ret_data.read::<u32>(rela_addr + 4 * 4);
                        ret_data.write::<usize>(*addr, *value as usize + APP_START_VA);
                    }
                    rela_addr += 24;
                }
            }
            Ok(".rela.plt") => {
                let mut rela_addr = section.address() as usize;
                let rela_end = section.size() as usize + rela_addr;
                let dynsym_addr = (elf.find_section_by_name(".dynsym").unwrap().address()) as usize;
                // rela.plt's entry is 24B = 6 * 4B, and entry[0] is offset, entry[3] is dynsym_index
                // dynsym's entry is 24B, and entry[0] is name_index
                while rela_addr < rela_end {
                    unsafe {
                        let addr = *ret_data.read::<u32>(rela_addr) as usize;
                        let dynsym_index = *ret_data.read::<u32>(rela_addr + 3 * 4) as usize;
                        let name_index = *ret_data.read::<u32>(dynsym_addr + dynsym_index * 24);
                        match elf.get_dyn_string(name_index) {
                            Ok("__libc_start_main") => {
                                // println!("libc");
                                ret_data.write::<usize>(
                                    addr,
                                    crate::mylibc::__libc_main_start as usize,
                                );
                            }
                            Ok("puts") => {
                                ret_data.write::<usize>(addr, mylibc::puts as usize);
                            }
                            Ok("sleep") => {
                                ret_data.write::<usize>(addr, mylibc::sleep as usize);
                            }
                            Ok(name) => {
                                panic!("unknown func name = {}", name);
                            }
                            _ => {}
                        }
                    }
                    rela_addr += 24;
                }
            }
            _ => {}
        }
    }
    ret_data
}
