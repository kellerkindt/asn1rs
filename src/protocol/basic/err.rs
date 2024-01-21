use asn1rs_model::asn::Tag;
use backtrace::Backtrace;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Range;

pub struct Error(pub(crate) Box<Inner>);

impl Error {
    #[inline]
    pub fn kind(&self) -> &ErrorKind {
        &self.0.kind
    }

    #[cold]
    #[inline(never)]
    pub fn unexpected_tag(expected: Tag, got: Tag) -> Self {
        Self::from(ErrorKind::UnexpectedTypeTag { expected, got })
    }

    #[cold]
    #[inline(never)]
    pub fn unexpected_length(expected: Range<u64>, got: u64) -> Self {
        Self::from(ErrorKind::UnexpectedTypeLength { expected, got })
    }

    #[cold]
    #[inline(never)]
    pub fn unexpected_choice_index(expected: Range<u64>, got: u64) -> Self {
        Self::from(ErrorKind::UnexpectedChoiceIndex { expected, got })
    }

    #[cold]
    #[inline(never)]
    pub fn unsupported_byte_len(max: u8, got: u8) -> Self {
        Self::from(ErrorKind::UnsupportedByteLen { max, got })
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Error(Box::new(Inner::from(kind)))
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(e: std::io::Error) -> Self {
        Self::from(ErrorKind::IoError(e))
    }
}

impl Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.0.kind)?;
        let mut backtrace = self.0.backtrace.clone();
        backtrace.resolve();
        writeln!(f, "{backtrace:?}")
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "encoding or decoding with basic rules failed"
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) kind: ErrorKind,
    pub(crate) backtrace: Backtrace,
}

impl From<ErrorKind> for Inner {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Self {
            kind,
            backtrace: Backtrace::new_unresolved(),
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    UnexpectedTypeTag { expected: Tag, got: Tag },
    UnexpectedTypeLength { expected: Range<u64>, got: u64 },
    UnexpectedChoiceIndex { expected: Range<u64>, got: u64 },
    UnsupportedByteLen { max: u8, got: u8 },
    IoError(std::io::Error),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::UnexpectedTypeTag { expected, got } => {
                write!(f, "Expected tag {expected:?} but got {got:?}")
            }
            ErrorKind::UnexpectedTypeLength { expected, got } => {
                write!(f, "Expected length in range {expected:?} but got {got:?}")
            }
            ErrorKind::UnexpectedChoiceIndex { expected, got } => {
                write!(f, "Expected choice index in {expected:?} but got {got:?}")
            }
            ErrorKind::UnsupportedByteLen { max, got } => {
                write!(
                    f,
                    "Unsupported byte length received, max={max:?} but got {got:?}"
                )
            }
            ErrorKind::IoError(e) => {
                write!(f, "Experienced underlying IO error: {e:?}")
            }
        }
    }
}
