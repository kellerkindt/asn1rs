#![deny(intra_doc_link_resolution_failure)]
#![warn(unused_extern_crates)]

#[cfg(feature = "psql")]
extern crate postgres;

pub mod converter;
pub mod gen;
pub mod io;
pub mod model;
pub mod parser;
