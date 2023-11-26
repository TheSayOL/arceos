#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(strict_provenance)]

use task::add_task;

mod config;
mod dl;
mod header;
mod mylibc;
mod page;
mod task;

// use axstd::println;

// 下一步, 支持多 app
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    dl::start_apps();
}
