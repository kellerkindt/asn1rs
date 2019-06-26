#![warn(unused_extern_crates)]
#![cfg_attr(all(feature = "bench_bit_buffer", test), feature(test))]

#[cfg(all(feature = "bench_bit_buffer", test))]
#[allow(unused_extern_crates)]
extern crate test;

#[cfg(feature = "psql")]
extern crate postgres;

pub mod converter;
pub mod gen;
pub mod io;
pub mod model;
pub mod parser;
