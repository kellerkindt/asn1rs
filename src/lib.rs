#![warn(unused_extern_crates)]
#![cfg(feature = "bench_bit_buffer")]
#![feature(test)]

#[cfg(feature = "bench_bit_buffer")]
#[allow(unused_extern_crates)]
extern crate test;

#[cfg(feature = "psql")]
extern crate postgres;

pub mod converter;
pub mod gen;
pub mod io;
pub mod model;
pub mod parser;
