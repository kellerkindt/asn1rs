use asn1rs_model::model::Tag;

#[derive(Debug, PartialOrd, PartialEq)]
pub enum Error {
    InvalidUtf8String,
    InvalidIa5String,
    UnsupportedOperation(String),
    InsufficientSpaceInDestinationBuffer,
    InsufficientDataInSourceBuffer,
    InvalidChoiceIndex(u64, u64),
    InvalidExtensionConstellation(bool, bool),
    ValueNotInRange(i64, i64, i64),
    ValueExceedsMaxInt,
    ValueIsNegativeButExpectedUnsigned(i64),
    SizeNotInRange(u64, u64, u64),
    OptFlagsExhausted,
    EndOfStream,
    InvalidType(Tag, Tag),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidUtf8String => {
                write!(f, "The underlying dataset is not a valid UTF8-String")
            }
            Error::InvalidIa5String => {
                write!(f, "The underlying dataset is not a valid IA5-String")
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
            Error::InvalidExtensionConstellation(expects, has) => write!(
                f,
                "Unexpected extension constellation, expected: {}, read: {}",
                expects, has
            ),
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
            Error::InvalidType(invalid_tag, valid_tag) => write!(
                f,
                "Got unexpected tag {:?} instead of {:?}",
                invalid_tag, valid_tag
            ),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "encoding or decoding UPER failed"
    }
}
