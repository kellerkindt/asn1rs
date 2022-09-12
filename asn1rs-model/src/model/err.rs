use std::error;
use std::fmt::{Debug, Display, Formatter};

use backtrace::Backtrace;

use crate::parser::Token;

#[derive(PartialOrd, PartialEq, Eq)]
pub enum ErrorKind {
    ExpectedText(Token),
    ExpectedTextGot(String, Token),
    ExpectedSeparator(Token),
    ExpectedSeparatorGot(char, Token),
    UnexpectedToken(Token),
    MissingModuleName,
    UnexpectedEndOfStream,
    InvalidRangeValue(Token),
    InvalidNumberForEnumVariant(Token),
    InvalidValueForConstant(Token),
    InvalidTag(Token),
    InvalidPositionForExtensionMarker(Token),
    InvalidIntText(Token),
    UnsupportedLiteral(Token),
    InvalidLiteral(Token),
}

pub struct Error {
    kind: ErrorKind,
    backtrace: Backtrace,
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error {
            kind,
            backtrace: Backtrace::new(),
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind)
    }
}

impl Error {
    pub fn invalid_int_value(token: Token) -> Self {
        ErrorKind::InvalidIntText(token).into()
    }

    pub fn invalid_position_for_extension_marker(token: Token) -> Self {
        ErrorKind::InvalidPositionForExtensionMarker(token).into()
    }

    pub fn invalid_tag(token: Token) -> Self {
        ErrorKind::InvalidTag(token).into()
    }

    pub fn invalid_value_for_constant(token: Token) -> Self {
        ErrorKind::InvalidValueForConstant(token).into()
    }

    pub fn invalid_number_for_enum_variant(token: Token) -> Self {
        ErrorKind::InvalidNumberForEnumVariant(token).into()
    }

    pub fn invalid_range_value(token: Token) -> Self {
        ErrorKind::InvalidRangeValue(token).into()
    }

    pub fn no_text(token: Token) -> Self {
        ErrorKind::ExpectedText(token).into()
    }

    pub fn expected_text(text: String, token: Token) -> Self {
        ErrorKind::ExpectedTextGot(text, token).into()
    }

    pub fn no_separator(token: Token) -> Self {
        ErrorKind::ExpectedSeparator(token).into()
    }

    pub fn expected_separator(separator: char, token: Token) -> Self {
        ErrorKind::ExpectedSeparatorGot(separator, token).into()
    }

    pub fn missing_module_name() -> Self {
        ErrorKind::MissingModuleName.into()
    }

    pub fn unexpected_token(token: Token) -> Self {
        ErrorKind::UnexpectedToken(token).into()
    }

    pub fn unexpected_end_of_stream() -> Self {
        ErrorKind::UnexpectedEndOfStream.into()
    }

    pub fn unsupported_value_reference_literal(token: Token) -> Self {
        ErrorKind::UnsupportedLiteral(token).into()
    }

    fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    pub fn token(&self) -> Option<&Token> {
        match &self.kind {
            ErrorKind::ExpectedText(t) => Some(t),
            ErrorKind::ExpectedTextGot(_, t) => Some(t),
            ErrorKind::ExpectedSeparator(t) => Some(t),
            ErrorKind::ExpectedSeparatorGot(_, t) => Some(t),
            ErrorKind::UnexpectedToken(t) => Some(t),
            ErrorKind::MissingModuleName => None,
            ErrorKind::UnexpectedEndOfStream => None,
            ErrorKind::InvalidRangeValue(t) => Some(t),
            ErrorKind::InvalidNumberForEnumVariant(t) => Some(t),
            ErrorKind::InvalidValueForConstant(t) => Some(t),
            ErrorKind::InvalidTag(t) => Some(t),
            ErrorKind::InvalidPositionForExtensionMarker(t) => Some(t),
            ErrorKind::InvalidIntText(t) => Some(t),
            ErrorKind::UnsupportedLiteral(t) => Some(t),
            ErrorKind::InvalidLiteral(t) => Some(t),
        }
    }
}

impl error::Error for Error {}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "{}", self)?;
        writeln!(f, "{:?}", self.backtrace())?;
        Ok(())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::ExpectedText(token) => write!(
                f,
                "At line {}, column {} expected text, but instead got: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            ErrorKind::ExpectedTextGot(text, token) => write!(
                f,
                "At line {}, column {} expected a text like \"{}\", but instead got: {}",
                token.location().line(),
                token.location().column(),
                text,
                token,
            ),
            ErrorKind::ExpectedSeparator(token) => write!(
                f,
                "At line {}, column {} expected separator, but instead got: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            ErrorKind::ExpectedSeparatorGot(separator, token) => write!(
                f,
                "At line {}, column {} expected a separator like '{}', but instead got: {}",
                token.location().line(),
                token.location().column(),
                separator,
                token,
            ),
            ErrorKind::UnexpectedToken(token) => write!(
                f,
                "At line {}, column {} an unexpected token was encountered: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            ErrorKind::MissingModuleName => {
                writeln!(f, "The ASN definition is missing the module name")
            }
            ErrorKind::UnexpectedEndOfStream => write!(f, "Unexpected end of stream or file"),
            ErrorKind::InvalidRangeValue(token) => write!(
                f,
                "At line {}, column {} an unexpected range value was encountered: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            ErrorKind::InvalidNumberForEnumVariant(token) => write!(
                f,
                "At line {}, column {} an invalid value for an enum variant was encountered: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            ErrorKind::InvalidValueForConstant(token) => write!(
                f,
                "At line {}, column {} an invalid value for an constant value was encountered: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            ErrorKind::InvalidTag(token) => write!(
                f,
                "At line {}, column {} an invalid value for a tag was encountered: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            ErrorKind::InvalidPositionForExtensionMarker(token) => write!(
                f,
                "At line {}, column {} an extension marker is present, which this is not allowed at that position",
                token.location().line(),
                token.location().column(),
            ),
            ErrorKind::InvalidIntText(token) => write!(
                f,
                "At line {}, column {} a number was expected but instead got: {}",
                token.location().line(),
                token.location().column(),
                token
            ),
            ErrorKind::UnsupportedLiteral(token) => write!(
                f,
                "At line {}, column {} an (yet) unsupported value reference literal was discovered: {}",
                token.location().line(),
                token.location().column(),
                token
            ),
            ErrorKind::InvalidLiteral(token) => write!(
                f,
                "At line {}, column {} an invalid literal was discovered: {}",
                token.location().line(),
                token.location().column(),
                token
            ),
        }
    }
}
