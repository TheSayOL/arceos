//! Runtime library of [ArceOS](https://github.com/rcore-os/arceos).
//!
//! Any application uses ArceOS should link this library. It does some
//! initialization work before entering the application's `main` function.

#![cfg_attr(not(test), no_std)]

pub mod util;
extern crate dtb;
