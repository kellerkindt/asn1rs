#![allow(dead_code)]
#![warn(unused_extern_crates)]

#[cfg(feature = "macros")]
pub extern crate asn1rs_macros as macros;

// provide an empty module, so that `use asn1rs::macros::*;` does not fail
#[cfg(not(feature = "macros"))]
pub mod macros {}

#[macro_use]
pub mod internal_macros;

#[macro_use]
extern crate serde_derive;

pub mod io;
pub mod prelude;
pub mod syn;

pub mod cli;
pub mod converter;

use asn1rs::converter::Converter;
pub use asn1rs_model::gen;
pub use asn1rs_model::model;
pub use asn1rs_model::parser;
use crate::cli::ConversionTarget;


pub fn main() {
    let params = <cli::Parameters as clap::Parser>::parse();
    let mut converter = Converter::default();

    for source in &params.source_files {
        if let Err(e) = converter.load_file(source) {
            println!("Failed to load file {}: {:?}", source, e);
            return;
        }
    }

    let result = match params.conversion_target {
        ConversionTarget::Rust =>  converter.to_rust(&params.destination_dir, |rust| {
            rust.set_fields_pub(!params.rust_fields_not_public);
            rust.set_fields_have_getter_and_setter(params.rust_getter_and_setter);
        }),
        ConversionTarget::Proto =>  converter.to_protobuf(&params.destination_dir),
    };

    match result {
        Err(e) => println!("Failed to convert: {:?}", e),
        Ok(files) => {
            for (source, mut files) in files {
                println!("Successfully converted {} => {}", source, files.remove(0));
                files
                    .iter()
                    .for_each(|f| println!("                          => {}", f));
            }
        }
    }
}
