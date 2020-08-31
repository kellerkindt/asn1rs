#![allow(dead_code)]
#![warn(unused_extern_crates)]

#[cfg(feature = "psql")]
extern crate postgres;

#[cfg(feature = "macros")]
pub extern crate asn1rs_macros as macros;

// provide an empty module, so that `use asn1rs::macros::*;` does not fail
#[cfg(not(feature = "macros"))]
pub mod macros {}

pub mod io;
pub mod prelude;
pub mod syn;

pub mod cli;
pub mod converter;

pub use asn1rs_model::gen;
pub use asn1rs_model::model;
pub use asn1rs_model::parser;

pub fn main() {
    let params = cli::parse_parameters();

    for source in &params.source_files {
        let result = match params.conversion_target.as_str() {
            cli::CONVERSION_TARGET_RUST => {
                converter::convert_to_rust(source, &params.destination_dir, |rust| {
                    rust.set_fields_pub(!params.rust_fields_not_public);
                    rust.set_fields_have_getter_and_setter(params.rust_getter_and_setter);
                })
            }
            cli::CONVERSION_TARGET_PROTO => {
                converter::convert_to_proto(source, &params.destination_dir)
            }
            cli::CONVERSION_TARGET_SQL => {
                converter::convert_to_sql(source, &params.destination_dir)
            }
            e => panic!("Unexpected CONVERSION_TARGET={}", e),
        };
        match result {
            Err(e) => println!("Failed to convert {}, reason: {:?}", source, e),
            Ok(mut files) => {
                println!("Successfully converted {} => {}", source, files.remove(0));
                files
                    .iter()
                    .for_each(|f| println!("                          => {}", f));
            }
        }
    }
}
