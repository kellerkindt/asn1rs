#![deny(rustdoc::broken_intra_doc_links)]
#![warn(unused_extern_crates)]

#[cfg(feature = "macros")]
pub extern crate asn1rs_macros as macros;

// provide an empty module, so that `use asn1rs::macros::*;` does not fail
#[cfg(not(feature = "macros"))]
pub mod macros {}

#[macro_use]
pub mod internal_macros;

pub mod descriptor;
pub mod prelude;
pub mod protocol;
pub mod rw;

#[cfg(feature = "model")]
pub mod converter;
#[cfg(feature = "model")]
pub use asn1rs_model as model;
