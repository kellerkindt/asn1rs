extern crate backtrace;
extern crate byteorder;
extern crate clap;
extern crate codegen;

mod converter;
mod gen;
mod io;
mod model;
mod parser;

use clap::{App, Arg};

pub fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Convertes (simple) .asn1 files to .rs files with UPER encoding")
        .arg(
            Arg::with_name("CONVERSION_TARGET")
                .required(true)
                .multiple(false)
                .value_name("CONVERSION_TARGET")
                .default_value("rust")
                .possible_values(&["rust", "proto"]),
        )
        .arg(
            Arg::with_name("DESTINATION_DIR")
                .required(true)
                .multiple(false)
                .value_name("DESTINATION_DIR"),
        )
        .arg(
            Arg::with_name("SOURCE_FILES")
                .required(true)
                .multiple(true)
                .value_name("SOURCE_FILES"),
        )
        .get_matches();

    let destination = matches.value_of("DESTINATION_DIR").unwrap();
    let sources = matches.values_of("SOURCE_FILES").unwrap();


    for source in sources {
        let result = match matches.value_of("CONVERSION_TARGET").unwrap() {
            "rust" => converter::convert_to_rust(source, destination),
            "proto" => converter::convert_to_proto(source, destination),
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
