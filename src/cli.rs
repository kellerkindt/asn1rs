use clap::AppSettings;
use clap::{App, Arg};

const ARG_RUST_FIELDS_NOT_PUBLIC: [&str; 5] = [
    "RUST_FIELDS_NOT_PUBLIC",
    "RUST_FIELDS_NOT_PUBLIC",
    "n",
    "rust-fields-not-public",
    "Whether the fields in the generated rust code are marked 'pub'",
];

const ARG_RUST_GETTER_AND_SETTER: [&str; 5] = [
    "RUST_GETTER_AND_SETTER",
    "RUST_GETTER_AND_SETTER",
    "g",
    "rust-getter-and-setter",
    "Whether to generate getter and setter for the fields of the generated rust structs",
];

const ARG_CONVERSION_TARGET: [&str; 5] = [
    "CONVERT_TO",
    "CONVERT_TO",
    "t",
    "convert-to",
    "The target to convert the input files to",
];

pub const CONVERSION_TARGET_RUST: &str = "rust";
pub const CONVERSION_TARGET_PROTO: &str = "proto";
pub const CONVERSION_TARGET_POSSIBLE_VALUES: [&str; 2] =
    [CONVERSION_TARGET_RUST, CONVERSION_TARGET_PROTO];

#[derive(Debug)]
pub struct Parameters {
    pub rust_fields_not_public: bool,
    pub rust_getter_and_setter: bool,
    pub conversion_target: String,
    pub source_files: Vec<String>,
    pub destination_dir: String,
}

pub fn arg<'a>(values: [&'a str; 5], default: Option<&'a str>) -> Arg<'a, 'a> {
    let mut arg = Arg::with_name(values[0])
        .env(values[0])
        .value_name(values[1])
        .short(values[2])
        .long(values[3])
        //.help(values[4])
        .takes_value(true);

    if let Some(default) = default {
        arg = arg.default_value(default);
    }

    arg
}

pub fn create_argument_parser<'a, 'b>() -> App<'a, 'b> {
    App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(AppSettings::ColoredHelp)
        .arg(arg(ARG_RUST_FIELDS_NOT_PUBLIC, None).takes_value(false))
        .arg(arg(ARG_RUST_GETTER_AND_SETTER, None).takes_value(false))
        .arg(
            arg(ARG_CONVERSION_TARGET, Some(CONVERSION_TARGET_RUST))
                .possible_values(&CONVERSION_TARGET_POSSIBLE_VALUES)
                .next_line_help(true),
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
}

pub fn parse_parameters() -> Parameters {
    let parser = create_argument_parser();
    let matches = parser.get_matches();
    Parameters {
        rust_fields_not_public: matches.is_present(ARG_RUST_FIELDS_NOT_PUBLIC[0]),
        rust_getter_and_setter: matches.is_present(ARG_RUST_GETTER_AND_SETTER[0]),
        conversion_target: matches
            .value_of_lossy(ARG_CONVERSION_TARGET[0])
            .expect("Missing conversion target")
            .to_string(),
        source_files: matches
            .values_of_lossy("SOURCE_FILES")
            .expect("Missing source files"),
        destination_dir: matches
            .value_of_lossy("DESTINATION_DIR")
            .expect("Missing destination directory")
            .to_string(),
    }
}
