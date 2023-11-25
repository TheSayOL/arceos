#![feature(asm_const)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

mod config;
mod dl;
mod header;
mod page;

use dl::start_apps;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    start_apps();
}
