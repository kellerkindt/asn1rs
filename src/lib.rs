#![deny(intra_doc_link_resolution_failure)]
#![warn(unused_extern_crates)]

#[cfg(feature = "psql")]
extern crate postgres;

#[cfg(feature = "macros")]
pub extern crate asn1rs_macros as macros;

#[cfg(feature = "model")]
pub mod converter;
#[cfg(feature = "model")]
pub use asn1rs_model::gen;
#[cfg(feature = "model")]
pub use asn1rs_model::model;
#[cfg(feature = "model")]
pub use asn1rs_model::parser;

pub mod io;
