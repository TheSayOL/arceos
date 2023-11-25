#![feature(asm_const)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

mod config;
mod dl;
mod header;
mod mylibc;
mod page;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    dl::start_apps();
}
