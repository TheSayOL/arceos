#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

mod config;
mod dl;
mod file;
mod mylibc;

use axstd::os::arceos::api::multi_app::*;
use axstd::vec::Vec;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let mut relocated_datas = Vec::new();
    for d in file::open_plash() {
        relocated_datas.push(dl::from_elf(d.as_slice()))
    }

    for rdata in relocated_datas.iter() {
        let entry = rdata.entry();
        let mut data = Vec::new();
        for s in rdata.data() {
            let mut v = Vec::new();
            for d in &s.data {
                v.push(*d);
            }
            let va = s.start_va;
            data.push((va, v));
        }
        create_task_from_data(data, entry);
    }
    join_all();
}
