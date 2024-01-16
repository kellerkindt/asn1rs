#![allow(dead_code)]
#![warn(unused_extern_crates)]

mod converter;
use converter::Converter;

pub fn main() {
    let params = <Parameters as clap::Parser>::parse();
    let mut converter = Converter::default();

    for source in &params.source_files {
        if let Err(e) = converter.load_file(source) {
            println!("Failed to load file {}: {:?}", source, e);
            return;
        }
    }

    let result = match params.conversion_target {
        ConversionTarget::Rust => converter.to_rust(&params.destination_dir, |rust| {
            rust.set_fields_pub(!params.rust_fields_not_public);
            rust.set_fields_have_getter_and_setter(params.rust_getter_and_setter);
        }),
        #[cfg(feature = "protobuf")]
        ConversionTarget::Proto => converter.to_protobuf(&params.destination_dir),
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

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
pub struct Parameters {
    #[arg(
        short = 'n',
        long = "rust-fields-not-public",
        env = "RUST_FIELDS_NOT_PUBLIC",
        help = "Whether the fields in the generated rust code are marked 'pub'"
    )]
    pub rust_fields_not_public: bool,
    #[arg(
        short = 'g',
        long = "rust-getter-and-setter",
        env = "RUST_GETTER_AND_SETTER",
        help = "Whether to generate getter and setter for the fields of the generated rust structs"
    )]
    pub rust_getter_and_setter: bool,
    #[arg(
        value_enum,
        short = 't',
        long = "convert-to",
        env = "CONVERT_TO",
        help = "The target to convert the input files to",
        default_value = "rust"
    )]
    pub conversion_target: ConversionTarget,
    #[arg(env = "DESTINATION_DIR")]
    pub destination_dir: String,
    #[arg(env = "SOURCE_FILES")]
    pub source_files: Vec<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum ConversionTarget {
    Rust,
    #[cfg(feature = "protobuf")]
    Proto,
}
