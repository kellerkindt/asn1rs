extern crate backtrace;
extern crate byteorder;
extern crate clap;
extern crate codegen;

mod converter;
mod gen;
mod io;
mod model;
mod parser;

use clap::{App, Arg, ArgMatches, SubCommand};

pub fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Convertes (simple) .asn1 files to .rs files with UPER encoding")
        .arg(
            Arg::with_name("SOURCE_FILES")
                .required(true)
                .multiple(true)
                .value_name("SOURCE_FILES"),
        )
        .arg(
            Arg::with_name("DESTINATION_DIR")
                .required(true)
                .multiple(false)
                .value_name("DESTINATION_DIR"),
        )
        .get_matches();

    let destination = matches.value_of("DESTINATION_DIR").unwrap();
    let sources = matches.values_of("SOURCE_FILES").unwrap();

    for source in sources {
        match converter::convert(source, destination) {
            Err(e) => println!("Failed to convert {}, reason: {:?}", source, e),
            Ok(_) => println!("Successfully converted {}", source),
        }
    }
}
