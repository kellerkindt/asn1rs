#![allow(dead_code)]
#![warn(unused_extern_crates)]

#[cfg(feature = "psql")]
extern crate postgres;

#[cfg(feature = "macros")]
pub extern crate asn1rs_macros as macros;

// provide an empty module, so that `use asn1rs::macros::*;` does not fail
#[cfg(not(feature = "macros"))]
pub mod macros {}

#[macro_use]
pub mod internal_macros;

#[macro_use]
pub extern crate serde_derive;

pub mod io;
pub mod prelude;
pub mod syn;

pub mod cli;
pub mod converter;

use asn1rs::converter::Converter;
pub use asn1rs_model::gen;
pub use asn1rs_model::model;
pub use asn1rs_model::parser;

pub fn main() {
    let params = cli::parse_parameters();
    let mut converter = Converter::default();

    for source in &params.source_files {
        if let Err(e) = converter.load_file(source) {
            println!("Failed to load file {}: {:?}", source, e);
            return;
        }
    }

    let result = match params.conversion_target.as_str() {
        cli::CONVERSION_TARGET_RUST => converter.to_rust(&params.destination_dir, |rust| {
            rust.set_fields_pub(!params.rust_fields_not_public);
            rust.set_fields_have_getter_and_setter(params.rust_getter_and_setter);
        }),
        cli::CONVERSION_TARGET_PROTO => converter.to_protobuf(&params.destination_dir),
        cli::CONVERSION_TARGET_SQL => converter.to_sql(&params.destination_dir),
        e => panic!("Unexpected CONVERSION_TARGET={}", e),
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
