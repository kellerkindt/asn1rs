use crate::model::Charset;
use std::string::FromUtf8Error;

#[derive(Debug, PartialEq)]
pub enum Error {
    FromUtf8Error(FromUtf8Error),
    InvalidString(Charset, char, usize),
    UnsupportedOperation(String),
    InsufficientSpaceInDestinationBuffer,
    InsufficientDataInSourceBuffer,
    InvalidChoiceIndex(u64, u64),
    ExtensionFieldsInconsistent(String),
    ValueNotInRange(i64, i64, i64),
    ValueExceedsMaxInt,
    ValueIsNegativeButExpectedUnsigned(i64),
    SizeNotInRange(u64, u64, u64),
    OptFlagsExhausted,
    EndOfStream,
}

impl Error {
    pub fn ensure_string_valid(charset: Charset, str: &str) -> Result<(), Self> {
        match charset.find_invalid(str) {
            None => Ok(()),
            Some((index, char)) => Err(Self::InvalidString(charset, char, index)),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FromUtf8Error(err) => {
                write!(f, "Failed to call String::from_utf8: ")?;
                err.fmt(f)
            }
            Error::InvalidString(charset, char, index) => {
                write!(
                    f,
                    "Invalid character for a string with the charset {:?} at index {}: {}",
                    charset, index, char
                )
            }
            Error::UnsupportedOperation(o) => write!(f, "The operation is not supported: {}", o),
            Error::InsufficientSpaceInDestinationBuffer => write!(
                f,
                "There is insufficient space in the destination buffer for this operation"
            ),
            Error::InsufficientDataInSourceBuffer => write!(
                f,
                "There is insufficient data in the source buffer for this operation"
            ),
            Error::InvalidChoiceIndex(index, variant_count) => write!(
                f,
                "Unexpected choice-index {} with variant count {}",
                index, variant_count
            ),
            Error::ExtensionFieldsInconsistent(name) => {
                write!(
                    f,
                    "The extension fields of {} are inconsistent, either all or none must be present",
                    name
                )
            }
            Error::ValueNotInRange(value, min, max) => write!(
                f,
                "The value {} is not within the inclusive range of {} and {}",
                value, min, max
            ),
            Error::ValueExceedsMaxInt => {
                write!(f, "The value exceeds the maximum supported integer size",)
            }
            Error::ValueIsNegativeButExpectedUnsigned(value) => write!(
                f,
                "The value {} is negative, but expected an unsigned/positive value",
                value
            ),
            Error::SizeNotInRange(size, min, max) => write!(
                f,
                "The size {} is not within the inclusive range of {} and {}",
                size, min, max
            ),
            Error::OptFlagsExhausted => write!(f, "All optional flags have already been exhausted"),
            Error::EndOfStream => write!(
                f,
                "Can no longer read or write any bytes from the underlying dataset"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "encoding or decoding UPER failed"
    }
}
