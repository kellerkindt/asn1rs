use crate::model::Charset;
use backtrace::Backtrace;
use std::string::FromUtf8Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Error(pub(crate) Box<Inner>);

impl Error {
    #[inline]
    pub fn kind(&self) -> &ErrorKind {
        &self.0.kind
    }

    #[cfg(feature = "descriptive-deserialize-errors")]
    pub fn scope_description(&self) -> &[crate::prelude::ScopeDescription] {
        &self.0.description[..]
    }
}

impl From<ErrorKind> for Error {
    #[cold]
    #[inline(never)]
    fn from(kind: ErrorKind) -> Self {
        Self(Box::new(Inner {
            kind,
            #[cfg(feature = "descriptive-deserialize-errors")]
            description: Vec::new(),
        }))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.kind)?;
        #[cfg(feature = "descriptive-deserialize-errors")]
        {
            writeln!(f)?;
            for desc in &self.0.description {
                writeln!(f, " - {desc:?}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "encoding or decoding UPER failed"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Inner {
    pub(crate) kind: ErrorKind,
    #[cfg(feature = "descriptive-deserialize-errors")]
    pub(crate) description: Vec<crate::syn::io::ScopeDescription>,
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    FromUtf8Error(FromUtf8Error),
    InvalidString(Charset, char, usize),
    UnsupportedOperation(String),
    InsufficientSpaceInDestinationBuffer(Backtrace),
    InsufficientDataInSourceBuffer(Backtrace),
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
    #[cold]
    #[inline(never)]
    pub fn ensure_string_valid(charset: Charset, str: &str) -> Result<(), Self> {
        match charset.find_invalid(str) {
            None => Ok(()),
            Some((index, char)) => Err(ErrorKind::InvalidString(charset, char, index).into()),
        }
    }

    #[cold]
    #[inline(never)]
    pub fn insufficient_space_in_destination_buffer() -> Self {
        ErrorKind::InsufficientSpaceInDestinationBuffer(Backtrace::new_unresolved()).into()
    }

    #[cold]
    #[inline(never)]
    pub fn insufficient_data_in_source_buffer() -> Self {
        ErrorKind::InsufficientDataInSourceBuffer(Backtrace::new_unresolved()).into()
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FromUtf8Error(err) => {
                write!(f, "Failed to call String::from_utf8: ")?;
                err.fmt(f)
            }
            Self::InvalidString(charset, char, index) => {
                write!(
                    f,
                    "Invalid character for a string with the charset {:?} at index {}: {}",
                    charset, index, char
                )
            }
            Self::UnsupportedOperation(o) => write!(f, "The operation is not supported: {}", o),
            Self::InsufficientSpaceInDestinationBuffer(backtrace) => write!(
                f,
                "There is insufficient space in the destination buffer for this operation:\n{:?}",
                {
                    let mut b = backtrace.clone();
                    b.resolve();
                    b
                }
            ),
            Self::InsufficientDataInSourceBuffer(backtrace) => write!(
                f,
                "There is insufficient data in the source buffer for this operation:\n{:?}",
                {
                    let mut b = backtrace.clone();
                    b.resolve();
                    b
                }
            ),
            Self::InvalidChoiceIndex(index, variant_count) => write!(
                f,
                "Unexpected choice-index {} with variant count {}",
                index, variant_count
            ),
            Self::ExtensionFieldsInconsistent(name) => {
                write!(
                    f,
                    "The extension fields of {} are inconsistent, either all or none must be present",
                    name
                )
            }
            Self::ValueNotInRange(value, min, max) => write!(
                f,
                "The value {} is not within the inclusive range of {} and {}",
                value, min, max
            ),
            Self::ValueExceedsMaxInt => {
                write!(f, "The value exceeds the maximum supported integer size",)
            }
            Self::ValueIsNegativeButExpectedUnsigned(value) => write!(
                f,
                "The value {} is negative, but expected an unsigned/positive value",
                value
            ),
            Self::SizeNotInRange(size, min, max) => write!(
                f,
                "The size {} is not within the inclusive range of {} and {}",
                size, min, max
            ),
            Self::OptFlagsExhausted => write!(f, "All optional flags have already been exhausted"),
            Self::EndOfStream => write!(
                f,
                "Can no longer read or write any bytes from the underlying dataset"
            ),
        }
    }
}

impl PartialEq for ErrorKind {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::FromUtf8Error(a) => matches!(other, Self::FromUtf8Error(oa) if a == oa),
            Self::InvalidString(a, b, c) => {
                matches!(other, Self::InvalidString(oa, ob, oc) if (a, b, c) == (oa, ob, oc))
            }
            Self::UnsupportedOperation(a) => {
                matches!(other, Self::UnsupportedOperation(oa) if a == oa)
            }
            Self::InsufficientSpaceInDestinationBuffer(_) => {
                matches!(other, Self::InsufficientSpaceInDestinationBuffer(_))
            }
            Self::InsufficientDataInSourceBuffer(_) => {
                matches!(other, Self::InsufficientDataInSourceBuffer(_))
            }
            Self::InvalidChoiceIndex(a, b) => {
                matches!(other, Self::InvalidChoiceIndex(oa, ob) if (a, b) == (oa, ob))
            }
            Self::ExtensionFieldsInconsistent(a) => {
                matches!(other, Self::ExtensionFieldsInconsistent(oa) if a == oa)
            }
            Self::ValueNotInRange(a, b, c) => {
                matches!(other, Self::ValueNotInRange(oa, ob, oc) if (a, b, c) == (oa, ob, oc))
            }
            Self::ValueExceedsMaxInt => matches!(other, Self::ValueExceedsMaxInt),
            Self::ValueIsNegativeButExpectedUnsigned(a) => {
                matches!(other, Self::ValueIsNegativeButExpectedUnsigned(oa) if a == oa)
            }
            Self::SizeNotInRange(a, b, c) => {
                matches!(other, Self::SizeNotInRange(oa, ob, oc) if (a,b ,c) == (oa, ob,oc))
            }
            Self::OptFlagsExhausted => matches!(other, Self::OptFlagsExhausted),
            Self::EndOfStream => matches!(other, Self::EndOfStream),
        }
    }
}
