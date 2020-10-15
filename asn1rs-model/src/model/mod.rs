pub mod protobuf;
pub mod rust;
pub mod sql;

pub use self::rust::Rust;
pub use self::rust::RustType;

pub use self::protobuf::Protobuf;
pub use self::protobuf::ProtobufType;

use crate::parser::Token;
use backtrace::Backtrace;
use std::convert::TryFrom;
use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Peekable;
use std::vec::IntoIter;

macro_rules! loop_ctrl_separator {
    ($token:expr) => {
        let token = $token;
        if token.eq_separator(',') {
            continue;
        } else if token.eq_separator('}') {
            break;
        } else {
            return Err(Error::unexpected_token(token));
        }
    };
}

#[derive(PartialOrd, PartialEq)]
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
        }
    }
}

impl StdError for Error {}

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
            )
        }
    }
}

/// The object-identifier is described in ITU-T X.680 | ISO/IEC 8824-1:2015
/// in chapter 32. The XML-related definitions as well as'DefinedValue' is
/// ignored by this implementation.
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ObjectIdentifier(Vec<ObjectIdentifierComponent>);

impl ObjectIdentifier {
    pub fn iter(&self) -> impl Iterator<Item = &ObjectIdentifierComponent> {
        self.0.iter()
    }
}

/// The object-identifier is described in ITU-T X.680 | ISO/IEC 8824-1:2015
/// in chapter 32. The XML-related definitions as well as'DefinedValue' is
/// ignored by this implementation.
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum ObjectIdentifierComponent {
    NameForm(String),
    NumberForm(u64),
    NameAndNumberForm(String, u64),
}

#[derive(Debug, Clone)]
pub struct Model<T> {
    pub name: String,
    pub oid: Option<ObjectIdentifier>,
    pub imports: Vec<Import>,
    pub definitions: Vec<Definition<T>>,
}

impl<T> Default for Model<T> {
    fn default() -> Self {
        Model {
            name: Default::default(),
            oid: None,
            imports: Default::default(),
            definitions: Default::default(),
        }
    }
}

impl Model<Asn> {
    pub fn try_from(value: Vec<Token>) -> Result<Self, Error> {
        let mut model = Model::default();
        let mut iter = value.into_iter().peekable();

        model.name = Self::read_name(&mut iter)?;
        model.oid = Self::maybe_read_oid(&mut iter)?;
        Self::skip_until_after_text_ignore_ascii_case(&mut iter, "BEGIN")?;

        while let Some(token) = iter.next() {
            if token.eq_text_ignore_ascii_case("END") {
                model.make_names_nice();
                return Ok(model);
            } else if token.eq_text_ignore_ascii_case("IMPORTS") {
                Self::read_imports(&mut iter)?
                    .into_iter()
                    .for_each(|i| model.imports.push(i));
            } else {
                model.definitions.push(Self::read_definition(
                    &mut iter,
                    token.into_text_or_else(Error::unexpected_token)?,
                )?);
            }
        }
        Err(Error::unexpected_end_of_stream())
    }

    fn read_name(iter: &mut Peekable<IntoIter<Token>>) -> Result<String, Error> {
        iter.next()
            .and_then(|token| token.into_text())
            .ok_or_else(Error::missing_module_name)
    }

    fn maybe_read_oid(
        iter: &mut Peekable<IntoIter<Token>>,
    ) -> Result<Option<ObjectIdentifier>, Error> {
        if iter.peek().map(|t| t.eq_separator('{')).unwrap_or(false) {
            let _token_start = iter.next().ok_or_else(Error::unexpected_end_of_stream)?;
            Ok(Some(Self::read_oid(iter)?))
        } else {
            Ok(None)
        }
    }

    fn read_oid(iter: &mut Peekable<IntoIter<Token>>) -> Result<ObjectIdentifier, Error> {
        let mut vec = Vec::default();
        while let Some(token) = iter.next() {
            if token.eq_separator('}') {
                break;
            } else if let Some(identifier) = token.text() {
                if identifier.chars().all(char::is_numeric) {
                    vec.push(ObjectIdentifierComponent::NumberForm(
                        identifier
                            .parse()
                            .map_err(|_| Error::invalid_int_value(token))?,
                    ));
                } else if iter.peek().map(|t| t.eq_separator('(')).unwrap_or(false) {
                    Self::next_separator_ignore_case(iter, '(')?;
                    let number = match Self::next_text(iter)?.parse::<u64>() {
                        Ok(number) => number,
                        Err(_) => return Err(Error::invalid_int_value(token)),
                    };
                    Self::next_separator_ignore_case(iter, ')')?;
                    vec.push(ObjectIdentifierComponent::NameAndNumberForm(
                        identifier.to_string(),
                        number,
                    ));
                } else {
                    vec.push(ObjectIdentifierComponent::NameForm(identifier.to_string()));
                }
            } else {
                return Err(Error::unexpected_token(token));
            }
        }
        Ok(ObjectIdentifier(vec))
    }

    fn skip_until_after_text_ignore_ascii_case(
        iter: &mut Peekable<IntoIter<Token>>,
        text: &str,
    ) -> Result<(), Error> {
        for t in iter {
            if t.eq_text_ignore_ascii_case(text) {
                return Ok(());
            }
        }
        Err(Error::unexpected_end_of_stream())
    }

    fn read_imports(iter: &mut Peekable<IntoIter<Token>>) -> Result<Vec<Import>, Error> {
        let mut imports = Vec::new();
        let mut import = Import::default();
        while let Some(token) = iter.next() {
            if token.eq_separator(';') {
                return Ok(imports);
            } else {
                let text = token.into_text_or_else(Error::unexpected_token)?;
                import.what.push(text);
                let token = Self::next(iter)?;
                if token.eq_separator(',') {
                    // ignore separator
                } else if token.eq_text_ignore_ascii_case("FROM") {
                    import.from = Self::next(iter)?.into_text_or_else(Error::unexpected_token)?;
                    import.from_oid = Self::maybe_read_oid(iter)?;
                    imports.push(import);
                    import = Import::default();
                }
            }
        }
        Err(Error::unexpected_end_of_stream())
    }
    fn read_definition(
        iter: &mut Peekable<IntoIter<Token>>,
        name: String,
    ) -> Result<Definition<Asn>, Error> {
        Self::next_separator_ignore_case(iter, ':')?;
        Self::next_separator_ignore_case(iter, ':')?;
        Self::next_separator_ignore_case(iter, '=')?;

        let (token, tag) = Self::next_with_opt_tag(iter)?;

        if token.eq_text_ignore_ascii_case("SEQUENCE") {
            Ok(Definition(
                name,
                Self::read_sequence_or_sequence_of(iter)?.opt_tagged(tag),
            ))
        } else if token.eq_text_ignore_ascii_case("SET") {
            Ok(Definition(
                name,
                Self::read_set_or_set_of(iter)?.opt_tagged(tag),
            ))
        } else if token.eq_text_ignore_ascii_case("ENUMERATED") {
            Ok(Definition(
                name,
                Type::Enumerated(Enumerated::try_from(iter)?).opt_tagged(tag),
            ))
        } else if token.eq_text_ignore_ascii_case("CHOICE") {
            Ok(Definition(
                name,
                Type::Choice(Choice::try_from(iter)?).opt_tagged(tag),
            ))
        } else if let Some(text) = token.text() {
            Ok(Definition(
                name,
                Self::read_role_given_text(iter, text.to_string())?.opt_tagged(tag),
            ))
        } else {
            Err(Error::unexpected_token(token))
        }
    }

    fn next_with_opt_tag(
        iter: &mut Peekable<IntoIter<Token>>,
    ) -> Result<(Token, Option<Tag>), Error> {
        let token = Self::next(iter)?;
        if token.eq_separator('[') {
            let tag = Tag::try_from(&mut *iter)?;
            Self::next_separator_ignore_case(iter, ']')?;
            let token = Self::next(iter)?;
            Ok((token, Some(tag)))
        } else {
            Ok((token, None))
        }
    }

    fn read_role(iter: &mut Peekable<IntoIter<Token>>) -> Result<Type, Error> {
        let text = Self::next_text(iter)?;
        Self::read_role_given_text(iter, text)
    }

    fn read_role_given_text(
        iter: &mut Peekable<IntoIter<Token>>,
        text: String,
    ) -> Result<Type, Error> {
        if text.eq_ignore_ascii_case("INTEGER") {
            Ok(Type::Integer(Integer::try_from(iter)?))
        } else if text.eq_ignore_ascii_case("BOOLEAN") {
            Ok(Type::Boolean)
        } else if text.eq_ignore_ascii_case("UTF8String") {
            Ok(Type::String(
                Model::<Asn>::maybe_read_size(iter)?,
                Charset::Utf8,
            ))
        } else if text.eq_ignore_ascii_case("IA5STring") {
            Ok(Type::String(
                Model::<Asn>::maybe_read_size(iter)?,
                Charset::Ia5,
            ))
        } else if text.eq_ignore_ascii_case("OCTET") {
            let token = Self::next(iter)?;
            if token.text().map_or(false, |t| t.eq("STRING")) {
                Ok(Type::OctetString(Model::<Asn>::maybe_read_size(iter)?))
            } else {
                Err(Error::unexpected_token(token))
            }
        } else if text.eq_ignore_ascii_case("BIT") {
            let token = Self::next(iter)?;
            if token.text().map_or(false, |t| t.eq("STRING")) {
                Ok(Type::BitString(BitString::try_from(iter)?))
            } else {
                Err(Error::unexpected_token(token))
            }
        } else if text.eq_ignore_ascii_case("CHOICE") {
            Ok(Type::Choice(Choice::try_from(iter)?))
        } else if text.eq_ignore_ascii_case("ENUMERATED") {
            Ok(Type::Enumerated(Enumerated::try_from(iter)?))
        } else if text.eq_ignore_ascii_case("SEQUENCE") {
            Ok(Self::read_sequence_or_sequence_of(iter)?)
        } else {
            Ok(Type::TypeReference(text, None))
        }
    }

    fn read_number_range(
        iter: &mut Peekable<IntoIter<Token>>,
    ) -> Result<Range<Option<i64>>, Error> {
        if Self::peek(iter)?.eq_separator('(') {
            Self::next_separator_ignore_case(iter, '(')?;
            let start = Self::next(iter)?;
            Self::next_separator_ignore_case(iter, '.')?;
            Self::next_separator_ignore_case(iter, '.')?;
            let end = Self::next(iter)?;
            let extensible = if Self::peek(iter)?.eq_separator(',') {
                let _ = Self::next_separator_ignore_case(iter, ',')?;
                Self::next_separator_ignore_case(iter, '.')?;
                Self::next_separator_ignore_case(iter, '.')?;
                Self::next_separator_ignore_case(iter, '.')?;
                true
            } else {
                false
            };
            Self::next_separator_ignore_case(iter, ')')?;
            let start = start
                .text()
                .filter(|txt| !txt.eq_ignore_ascii_case("MIN"))
                .map(|t| t.parse::<i64>())
                .transpose()
                .map_err(|_| Error::invalid_range_value(start))?;

            let end = end
                .text()
                .filter(|txt| !txt.eq_ignore_ascii_case("MAX"))
                .map(|t| t.parse::<i64>())
                .transpose()
                .map_err(|_| Error::invalid_range_value(end))?;

            match (start, end) {
                (Some(0), None) => Ok(Range(None, None, extensible)),
                (start, end) => Ok(Range(start, end, extensible)),
            }
        } else {
            Ok(Range(None, None, false))
        }
    }

    fn maybe_read_size(iter: &mut Peekable<IntoIter<Token>>) -> Result<Size, Error> {
        if Self::peek(iter)?.eq_separator('(') {
            Self::next_separator_ignore_case(iter, '(')?;
            let result = Self::read_size(iter)?;
            Self::next_separator_ignore_case(iter, ')')?;
            Ok(result)
        } else if Self::peek(iter)?.eq_text_ignore_ascii_case("SIZE") {
            Self::read_size(iter)
        } else {
            Ok(Size::Any)
        }
    }

    fn read_size(iter: &mut Peekable<IntoIter<Token>>) -> Result<Size, Error> {
        let size_token = Self::next(iter)?;
        if size_token.eq_text_ignore_ascii_case("SIZE") {
            Self::next_separator_ignore_case(iter, '(')?;
            let start = Self::next(iter)?;
            let start = start
                .text()
                .filter(|txt| !txt.eq_ignore_ascii_case("MIN"))
                .map(|t| t.parse::<usize>())
                .transpose()
                .map_err(|_| Error::invalid_range_value(start))?;

            if !Self::peek(iter)?.eq_separator('.') {
                match Self::next(iter)? {
                    t if t.eq_separator(')') => Ok(Size::Fix(start.unwrap_or_default(), false)),
                    t if t.eq_separator(',') => {
                        Self::next_separator_ignore_case(iter, '.')?;
                        Self::next_separator_ignore_case(iter, '.')?;
                        Self::next_separator_ignore_case(iter, '.')?;
                        Ok(Size::Fix(start.unwrap_or_default(), true))
                    }
                    t => Err(Error::unexpected_token(t)),
                }
            } else {
                Self::next_separator_ignore_case(iter, '.')?;
                Self::next_separator_ignore_case(iter, '.')?;
                let end = Self::next(iter)?;
                let end = end
                    .text()
                    .filter(|txt| !txt.eq_ignore_ascii_case("MAX"))
                    .map(|t| t.parse::<usize>())
                    .transpose()
                    .map_err(|_| Error::invalid_range_value(end))?;

                const MAX: usize = i64::MAX as usize;
                let any = matches!(
                    (start, end),
                    (None, None) | (Some(0), None) | (None, Some(MAX))
                );

                if any {
                    Self::next_separator_ignore_case(iter, ')')?;
                    Ok(Size::Any)
                } else {
                    let start = start.unwrap_or_default();
                    let end = end.unwrap_or_else(|| i64::MAX as usize);
                    let extensible = if Self::peek(iter)?.eq_separator(',') {
                        let _ = Self::next_separator_ignore_case(iter, ',')?;
                        Self::next_separator_ignore_case(iter, '.')?;
                        Self::next_separator_ignore_case(iter, '.')?;
                        Self::next_separator_ignore_case(iter, '.')?;
                        true
                    } else {
                        false
                    };
                    Self::next_separator_ignore_case(iter, ')')?;
                    if start == end {
                        Ok(Size::Fix(start, extensible))
                    } else {
                        Ok(Size::Range(start, end, extensible))
                    }
                }
            }
        } else {
            Err(Error::unexpected_token(size_token))
        }
    }

    fn constant_i64_parser(token: Token) -> Result<i64, Error> {
        let parsed = token.text().and_then(|s| s.parse().ok());
        parsed.ok_or_else(|| Error::invalid_value_for_constant(token))
    }

    fn constant_u64_parser(token: Token) -> Result<u64, Error> {
        let parsed = token.text().and_then(|s| s.parse().ok());
        parsed.ok_or_else(|| Error::invalid_value_for_constant(token))
    }

    fn maybe_read_constants<T, F: Fn(Token) -> Result<T, Error>>(
        iter: &mut Peekable<IntoIter<Token>>,
        parser: F,
    ) -> Result<Vec<(String, T)>, Error> {
        let mut constants = Vec::default();
        if Self::peek(iter)?.eq_separator('{') {
            Self::next_separator_ignore_case(iter, '{')?;
            loop {
                constants.push(Self::read_constant(iter, |token| parser(token))?);
                if !Self::peek(iter)?.eq_separator(',') {
                    break;
                } else {
                    let _ = Self::next(iter)?;
                }
            }
            Self::next_separator_ignore_case(iter, '}')?;
        }
        Ok(constants)
    }

    fn read_constant<T, F: Fn(Token) -> Result<T, Error>>(
        iter: &mut Peekable<IntoIter<Token>>,
        parser: F,
    ) -> Result<(String, T), Error> {
        let name = Self::next_text(iter)?;
        Self::next_separator_ignore_case(iter, '(')?;
        let value = Self::next(iter)?;
        Self::next_separator_ignore_case(iter, ')')?;
        Ok((name, parser(value)?))
    }

    fn read_sequence_or_sequence_of(iter: &mut Peekable<IntoIter<Token>>) -> Result<Type, Error> {
        let size = Self::maybe_read_size(iter)?;
        let token = Self::peek(iter)?;

        if token.eq_text_ignore_ascii_case("OF") {
            let _ = Self::next(iter)?;
            Ok(Type::SequenceOf(Box::new(Self::read_role(iter)?), size))
        } else if token.eq_separator('{') {
            Ok(Type::Sequence(ComponentTypeList::try_from(iter)?))
        } else {
            Err(Error::unexpected_token(Self::next(iter)?))
        }
    }

    fn read_set_or_set_of(iter: &mut Peekable<IntoIter<Token>>) -> Result<Type, Error> {
        let size = Self::maybe_read_size(iter)?;
        let token = Self::peek(iter)?;

        if token.eq_text_ignore_ascii_case("OF") {
            let _ = Self::next(iter)?;
            Ok(Type::SetOf(Box::new(Self::read_role(iter)?), size))
        } else if token.eq_separator('{') {
            Ok(Type::Set(ComponentTypeList::try_from(iter)?))
        } else {
            Err(Error::unexpected_token(Self::next(iter)?))
        }
    }

    fn read_field(iter: &mut Peekable<IntoIter<Token>>) -> Result<(Field<Asn>, bool), Error> {
        let name = Self::next_text(iter)?;
        let (token, tag) = Self::next_with_opt_tag(iter)?;
        let mut field = Field {
            name,
            role: Self::read_role_given_text(iter, token.into_text_or_else(Error::no_text)?)?
                .opt_tagged(tag),
        };
        let mut token = Self::next(iter)?;
        if let Some(_optional_flag) = token.text().map(|s| s.eq_ignore_ascii_case("OPTIONAL")) {
            field.role.optional();
            token = Self::next(iter)?;
        }

        let (continues, ends) = token
            .separator()
            .map_or((false, false), |s| (s == ',', s == '}'));

        if continues || ends {
            Ok((field, continues))
        } else {
            Err(Error::unexpected_token(token))
        }
    }

    fn next(iter: &mut Peekable<IntoIter<Token>>) -> Result<Token, Error> {
        iter.next().ok_or_else(Error::unexpected_end_of_stream)
    }

    fn peek(iter: &mut Peekable<IntoIter<Token>>) -> Result<&Token, Error> {
        iter.peek().ok_or_else(Error::unexpected_end_of_stream)
    }

    fn next_text(iter: &mut Peekable<IntoIter<Token>>) -> Result<String, Error> {
        Self::next(iter)?.into_text_or_else(Error::no_text)
    }

    fn next_separator_ignore_case(
        iter: &mut Peekable<IntoIter<Token>>,
        separator: char,
    ) -> Result<(), Error> {
        let token = Self::next(iter)?;
        if let Some(token) = token.separator() {
            if token.eq_ignore_ascii_case(&separator) {
                return Ok(());
            }
        }
        Err(Error::expected_separator(separator, token))
    }

    pub fn make_names_nice(&mut self) {
        Self::make_name_nice(&mut self.name);
        for import in &mut self.imports {
            Self::make_name_nice(&mut import.from);
        }
    }

    fn make_name_nice(name: &mut String) {
        const TO_REMOVE_AT_END: &[&str] = &["_Module", "Module"];
        for to_remove in TO_REMOVE_AT_END.iter() {
            if name.ends_with(to_remove) {
                let new_len = name.len() - to_remove.len();
                name.truncate(new_len);
            }
        }
    }

    pub fn to_rust(&self) -> Model<rust::Rust> {
        let scope: &[&Self] = &[];
        Model::convert_asn_to_rust(self, scope)
    }

    pub fn to_rust_with_scope(&self, scope: &[&Self]) -> Model<rust::Rust> {
        Model::convert_asn_to_rust(self, scope)
    }
}

#[derive(Debug, Default, Clone, PartialOrd, PartialEq)]
pub struct Import {
    pub what: Vec<String>,
    pub from: String,
    pub from_oid: Option<ObjectIdentifier>,
}

pub struct TagResolver<'a> {
    model: &'a Model<Asn>,
    scope: &'a [&'a Model<Asn>],
}

impl TagResolver<'_> {
    pub fn resolve_default(ty: &Type) -> Option<Tag> {
        let model = Model::<Asn>::default();
        TagResolver {
            model: &model,
            scope: &[],
        }
        .resolve_type_tag(ty)
    }

    /// ITU-T X.680 | ISO/IEC 8824-1, 8.6
    /// ITU-T X.680 | ISO/IEC 8824-1, 41, table 8
    pub fn resolve_tag(&self, ty: &str) -> Option<Tag> {
        self.model
            .imports
            .iter()
            .find(|import| import.what.iter().any(|what| what.eq(ty)))
            .map(|import| &import.from)
            .and_then(|model_name| self.scope.iter().find(|model| model.name.eq(model_name)))
            .and_then(|model| {
                TagResolver {
                    model,
                    scope: self.scope,
                }
                .resolve_tag(ty)
            })
            .or_else(|| {
                self.model.definitions.iter().find(|d| d.0.eq(ty)).and_then(
                    |Definition(_name, asn)| asn.tag.or_else(|| self.resolve_type_tag(&asn.r#type)),
                )
            })
    }

    /// ITU-T X.680 | ISO/IEC 8824-1, 8.6
    /// ITU-T X.680 | ISO/IEC 8824-1, 41, table 8
    pub fn resolve_no_default(&self, ty: &Type) -> Option<Tag> {
        let default = Self::resolve_default(ty);
        let resolved = self.resolve_type_tag(ty);
        resolved.filter(|r| default.ne(&Some(*r)))
    }

    /// ITU-T X.680 | ISO/IEC 8824-1, 8.6
    /// ITU-T X.680 | ISO/IEC 8824-1, 41, table 8
    pub fn resolve_type_tag(&self, ty: &Type) -> Option<Tag> {
        match ty {
            Type::Boolean => Some(Tag::DEFAULT_BOOLEAN),
            Type::Integer(_) => Some(Tag::DEFAULT_INTEGER),
            Type::BitString(_) => Some(Tag::DEFAULT_BIT_STRING),
            Type::OctetString(_) => Some(Tag::DEFAULT_OCTET_STRING),
            Type::Enumerated(_) => Some(Tag::DEFAULT_ENUMERATED),
            Type::String(_, Charset::Utf8) => Some(Tag::DEFAULT_UTF8_STRING),
            Type::String(_, Charset::Ia5) => Some(Tag::DEFAULT_IA5_STRING),
            Type::Optional(inner) => self.resolve_type_tag(&**inner),
            Type::Sequence(_) => Some(Tag::DEFAULT_SEQUENCE),
            Type::SequenceOf(_, _) => Some(Tag::DEFAULT_SEQUENCE_OF),
            Type::Set(_) => Some(Tag::DEFAULT_SET),
            Type::SetOf(_, _) => Some(Tag::DEFAULT_SET_OF),
            Type::Choice(choice) => {
                let mut tags = choice
                    .variants()
                    .take(
                        choice
                            .extension_after
                            .map(|extension_after| extension_after + 1)
                            .unwrap_or(choice.variants.len()),
                    )
                    .map(|v| v.tag().or_else(|| self.resolve_type_tag(v.r#type())))
                    .collect::<Option<Vec<Tag>>>()?;
                tags.sort();
                if cfg!(feature = "debug-proc-macro") {
                    println!("resolved::::{:?}", tags);
                }
                tags.into_iter().next()
            }
            Type::TypeReference(inner, tag) => {
                let tag = tag.clone().or_else(|| self.resolve_tag(inner.as_str()));
                if cfg!(feature = "debug-proc-macro") {
                    println!("resolved :: {}::Tag = {:?}", inner, tag);
                }
                tag
            }
        }
    }
}

pub struct Context<'a> {
    resolver: TagResolver<'a>,
    target: &'a mut Vec<Definition<Rust>>,
}

impl Context<'_> {
    pub fn add_definition(&mut self, def: Definition<Rust>) {
        self.target.push(def)
    }

    pub fn resolver(&self) -> &TagResolver<'_> {
        &self.resolver
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum Size {
    Any,
    Fix(usize, bool),
    Range(usize, usize, bool),
}

impl Size {
    pub fn min(&self) -> Option<usize> {
        match self {
            Size::Any => None,
            Size::Fix(min, _) => Some(*min),
            Size::Range(min, _, _) => Some(*min),
        }
    }

    pub fn max(&self) -> Option<usize> {
        match self {
            Size::Any => None,
            Size::Fix(max, _) => Some(*max),
            Size::Range(_, max, _) => Some(*max),
        }
    }

    pub fn extensible(&self) -> bool {
        match self {
            Size::Any => false,
            Size::Fix(_, extensible) => *extensible,
            Size::Range(_, _, extensible) => *extensible,
        }
    }

    pub fn to_constraint_string(&self) -> Option<String> {
        if Size::Any != *self {
            Some(format!(
                "{}..{}{}",
                self.min().unwrap_or_default(),
                self.max().unwrap_or_else(|| i64::max_value() as usize),
                if self.extensible() { ",..." } else { "" }
            ))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum Charset {
    Utf8,
    Ia5,
}

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq)]
pub struct Range<T>(pub T, pub T, bool);

impl<T> Range<T> {
    pub const fn inclusive(min: T, max: T) -> Self {
        Self(min, max, false)
    }

    pub fn with_extensible(self, extensible: bool) -> Self {
        let Range(min, max, _) = self;
        Range(min, max, extensible)
    }

    pub const fn min(&self) -> &T {
        &self.0
    }

    pub const fn max(&self) -> &T {
        &self.1
    }

    pub const fn extensible(&self) -> bool {
        self.2
    }

    pub fn wrap_opt(self) -> Range<Option<T>> {
        let Range(min, max, extensible) = self;
        Range(Some(min), Some(max), extensible)
    }
}

impl<T: Copy> Range<Option<T>> {
    pub fn none() -> Self {
        Range(None, None, false)
    }

    pub fn min_max(&self, min_fn: impl Fn() -> T, max_fn: impl Fn() -> T) -> Option<(T, T)> {
        match (self.0, self.1) {
            (Some(min), Some(max)) => Some((min, max)),
            (Some(min), None) => Some((min, max_fn())),
            (None, Some(max)) => Some((min_fn(), max)),
            (None, None) => None,
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Definition<T>(pub String, pub T);

impl<T> Definition<T> {
    #[cfg(test)]
    pub fn new<I: ToString>(name: I, value: T) -> Self {
        Definition(name.to_string(), value)
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn value(&self) -> &T {
        &self.1
    }
}

impl TagProperty for Definition<Asn> {
    fn tag(&self) -> Option<Tag> {
        self.1.tag()
    }

    fn set_tag(&mut self, tag: Tag) {
        self.1.set_tag(tag)
    }

    fn reset_tag(&mut self) {
        self.1.reset_tag()
    }
}

impl TagProperty for Definition<Rust> {
    fn tag(&self) -> Option<Tag> {
        self.1.tag()
    }

    fn set_tag(&mut self, tag: Tag) {
        self.1.set_tag(tag)
    }

    fn reset_tag(&mut self) {
        self.1.reset_tag()
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Field<T> {
    pub name: String,
    pub role: T,
}

impl<T: TagProperty> TagProperty for Field<T> {
    fn tag(&self) -> Option<Tag> {
        self.role.tag()
    }

    fn set_tag(&mut self, tag: Tag) {
        self.role.set_tag(tag)
    }

    fn reset_tag(&mut self) {
        self.role.reset_tag()
    }
}

///ITU-T X.680 | ISO/IEC 8824-1, chapter 8
///
/// # Ordering
/// According to ITU-T X.680 | ISO/IEC 8824-1, 8.6, the canonical order is
/// a) Universal, Application, ContextSpecific and Private and
/// b) within each class, the numbers shall be ordered ascending
///
/// ```rust
/// use asn1rs_model::model::Tag;
/// let mut tags = vec![
///     Tag::Universal(1),
///     Tag::Application(0),
///     Tag::Private(7),
///     Tag::ContextSpecific(107),
///     Tag::ContextSpecific(32),
///     Tag::Universal(0),
/// ];
/// tags.sort();
/// assert_eq!(tags, vec![
///     Tag::Universal(0),
///     Tag::Universal(1),
///     Tag::Application(0),
///     Tag::ContextSpecific(32),
///     Tag::ContextSpecific(107),
///     Tag::Private(7),
/// ]);
/// ```
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum Tag {
    Universal(usize),
    Application(usize),
    ContextSpecific(usize),
    Private(usize),
}

impl Tag {
    pub const DEFAULT_BOOLEAN: Tag = Tag::Universal(1);
    pub const DEFAULT_INTEGER: Tag = Tag::Universal(2);
    pub const DEFAULT_BIT_STRING: Tag = Tag::Universal(3);
    pub const DEFAULT_OCTET_STRING: Tag = Tag::Universal(4);
    pub const DEFAULT_NULL: Tag = Tag::Universal(5);
    pub const DEFAULT_OBJECT_IDENTIFIER: Tag = Tag::Universal(6);
    pub const DEFAULT_OBJECT_DESCRIPTOR: Tag = Tag::Universal(7);
    pub const DEFAULT_EXTERNAL: Tag = Tag::Universal(8);
    pub const DEFAULT_REAL: Tag = Tag::Universal(9);
    pub const DEFAULT_ENUMERATED: Tag = Tag::Universal(10);
    pub const DEFAULT_EMBEDDED_PDV: Tag = Tag::Universal(11);
    pub const DEFAULT_UTF8_STRING: Tag = Tag::Universal(12);
    pub const DEFAULT_RELATIVE_OBJECT_IDENTIFIER: Tag = Tag::Universal(13);
    pub const DEFAULT_TIME: Tag = Tag::Universal(14);
    pub const DEFAULT_SEQUENCE: Tag = Tag::Universal(16);
    pub const DEFAULT_SEQUENCE_OF: Tag = Tag::Universal(16);
    pub const DEFAULT_SET: Tag = Tag::Universal(17);
    pub const DEFAULT_SET_OF: Tag = Tag::Universal(17);
    pub const DEFAULT_NUMERIC_STRING: Tag = Tag::Universal(18);
    pub const DEFAULT_PRINTABLE_STRING: Tag = Tag::Universal(19);
    pub const DEFAULT_T61_STRING: Tag = Tag::Universal(20);
    pub const DEFAULT_VIDEOTEX_STRING: Tag = Tag::Universal(21);
    pub const DEFAULT_IA5_STRING: Tag = Tag::Universal(22);
    pub const DEFAULT_UTC_TIME: Tag = Tag::Universal(23);
    pub const DEFAULT_GENERALIZED_TIME: Tag = Tag::Universal(24);
    pub const DEFAULT_GRAPHIC_STRING: Tag = Tag::Universal(25);
    pub const DEFAULT_VISIBLE_STRING: Tag = Tag::Universal(26);
    pub const DEFAULT_GENERAL_STRING: Tag = Tag::Universal(27);
    pub const DEFAULT_UNIVERSAL_STRING: Tag = Tag::Universal(28);
    pub const DEFAULT_CHARACTER_STRING: Tag = Tag::Universal(29);
    pub const DEFAULT_BMP_STRING: Tag = Tag::Universal(30);
    pub const DEFAULT_DATE: Tag = Tag::Universal(31);
    pub const DEFAULT_TIME_OF_DAY: Tag = Tag::Universal(32);
    pub const DEFAULT_DATE_TIME: Tag = Tag::Universal(33);
    pub const DEFAULT_DURATION: Tag = Tag::Universal(34);
    pub const DEFAULT_OBJECT_IDENTIFIER_IRI: Tag = Tag::Universal(35);
    pub const DEFAULT_RELATIVE_OBJECT_IDENTIFIER_IRI: Tag = Tag::Universal(36);
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Tag::Universal(1) => f.write_str("BOOLEAN"),
            Tag::Universal(2) => f.write_str("INTEGER"),
            Tag::Universal(3) => f.write_str("BIT_STRING"),
            Tag::Universal(4) => f.write_str("OCTET_STRING"),
            Tag::Universal(5) => f.write_str("NULL"),
            Tag::Universal(6) => f.write_str("OBJECT_IDENTIFIER"),
            Tag::Universal(7) => f.write_str("OBJECT_DESCRIPTOR"),
            Tag::Universal(8) => f.write_str("EXTERNAL"),
            Tag::Universal(9) => f.write_str("REAL"),
            Tag::Universal(10) => f.write_str("ENUMERATED"),
            Tag::Universal(11) => f.write_str("EMBEDDED_PDV"),
            Tag::Universal(12) => f.write_str("UTF8_STRING"),
            Tag::Universal(13) => f.write_str("RELATIVE_OBJECT_IDENTIFIER"),
            Tag::Universal(14) => f.write_str("TIME"),
            Tag::Universal(16) => f.write_str("SEQUENCE"),
            Tag::Universal(17) => f.write_str("SET"),
            Tag::Universal(18) => f.write_str("NUMERIC_STRING"),
            Tag::Universal(19) => f.write_str("PRINTABLE_STRING"),
            Tag::Universal(20) => f.write_str("T61_STRING"),
            Tag::Universal(21) => f.write_str("VIDEOTEX_STRING"),
            Tag::Universal(22) => f.write_str("IA5_STRING"),
            Tag::Universal(23) => f.write_str("UTC_TIME"),
            Tag::Universal(24) => f.write_str("GENERALIZED_TIME"),
            Tag::Universal(25) => f.write_str("GRAPHIC_STRING"),
            Tag::Universal(26) => f.write_str("VISIBLE_STRING"),
            Tag::Universal(27) => f.write_str("GENERAL_STRING"),
            Tag::Universal(28) => f.write_str("UNIVERSAL_STRING"),
            Tag::Universal(29) => f.write_str("CHARACTER_STRING"),
            Tag::Universal(30) => f.write_str("BMP_STRING"),
            Tag::Universal(31) => f.write_str("DATE"),
            Tag::Universal(32) => f.write_str("TIME_OF_DAY"),
            Tag::Universal(33) => f.write_str("DATE_TIME"),
            Tag::Universal(34) => f.write_str("DURATION"),
            Tag::Universal(35) => f.write_str("OBJECT_IDENTIFIER_IRI"),
            Tag::Universal(36) => f.write_str("RELATIVE_OBJECT_IDENTIFIER_IRI"),
            _ => f.write_str(&*format!("{:?}", self)),
        }
    }
}

impl TryFrom<&mut Peekable<IntoIter<Token>>> for Tag {
    type Error = Error;

    fn try_from(iter: &mut Peekable<IntoIter<Token>>) -> Result<Self, Self::Error> {
        macro_rules! parse_tag_number {
            () => {
                parse_tag_number!(Model::<Asn>::next(iter)?)
            };
            ($tag:expr) => {{
                let tag = $tag;
                tag.text()
                    .and_then(|t| t.parse().ok())
                    .ok_or_else(|| Error::invalid_tag(tag))?
            }};
        }

        let number_or_class = Model::<Asn>::next(iter)?;

        if let Some(text) = number_or_class.text() {
            Ok(match text {
                "UNIVERSAL" => Tag::Universal(parse_tag_number!()),
                "APPLICATION" => Tag::Application(parse_tag_number!()),
                "PRIVATE" => Tag::Private(parse_tag_number!()),
                _context_specific => Tag::ContextSpecific(parse_tag_number!(number_or_class)),
            })
        } else {
            Err(Error::no_text(number_or_class))
        }
    }
}

pub trait TagProperty: Sized {
    fn tag(&self) -> Option<Tag>;

    fn set_tag(&mut self, tag: Tag);

    fn reset_tag(&mut self);

    fn with_tag_opt(self, tag: Option<Tag>) -> Self
    where
        Self: Sized,
    {
        if let Some(tag) = tag {
            self.with_tag(tag)
        } else {
            self.without_tag()
        }
    }

    fn with_tag(mut self, tag: Tag) -> Self
    where
        Self: Sized,
    {
        self.set_tag(tag);
        self
    }

    fn without_tag(mut self) -> Self
    where
        Self: Sized,
    {
        self.reset_tag();
        self
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Asn {
    pub tag: Option<Tag>,
    pub r#type: Type,
}

impl Asn {
    pub fn optional(&mut self) {
        let optional = self.r#type.clone().optional();
        self.r#type = optional;
    }

    pub const fn opt_tagged(tag: Option<Tag>, r#type: Type) -> Self {
        Self { tag, r#type }
    }

    pub const fn untagged(r#type: Type) -> Self {
        Self::opt_tagged(None, r#type)
    }

    pub const fn tagged(tag: Tag, r#type: Type) -> Self {
        Self::opt_tagged(Some(tag), r#type)
    }
}

impl From<Type> for Asn {
    fn from(r#type: Type) -> Self {
        Self::untagged(r#type)
    }
}

impl TagProperty for Asn {
    fn tag(&self) -> Option<Tag> {
        self.tag
    }

    fn set_tag(&mut self, tag: Tag) {
        self.tag = Some(tag)
    }

    fn reset_tag(&mut self) {
        self.tag = None
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Type {
    Boolean,
    Integer(Integer),
    String(Size, Charset),
    OctetString(Size),
    BitString(BitString),

    Optional(Box<Type>),

    Sequence(ComponentTypeList),
    SequenceOf(Box<Type>, Size),
    Set(ComponentTypeList),
    SetOf(Box<Type>, Size),
    Enumerated(Enumerated),
    Choice(Choice),
    TypeReference(String, Option<Tag>),
}

impl Type {
    pub const fn unconstrained_utf8string() -> Self {
        Self::String(Size::Any, Charset::Utf8)
    }

    pub const fn unconstrained_octetstring() -> Self {
        Self::OctetString(Size::Any)
    }

    pub fn unconstrained_integer() -> Self {
        Self::integer_with_range_opt(Range::none())
    }

    pub const fn integer_with_range(range: Range<Option<i64>>) -> Self {
        Self::Integer(Integer {
            range,
            constants: Vec::new(),
        })
    }

    pub const fn integer_with_range_opt(range: Range<Option<i64>>) -> Self {
        Self::Integer(Integer {
            range,
            constants: Vec::new(),
        })
    }

    pub const fn bit_vec_with_size(size: Size) -> Self {
        Self::BitString(BitString {
            size,
            constants: Vec::new(),
        })
    }

    pub const fn sequence_from_fields(fields: Vec<Field<Asn>>) -> Self {
        Self::Sequence(ComponentTypeList {
            fields,
            extension_after: None,
        })
    }

    pub fn choice_from_variants(variants: Vec<ChoiceVariant>) -> Self {
        Self::Choice(Choice::from(variants))
    }

    pub fn optional(self) -> Self {
        Self::Optional(Box::new(self))
    }

    pub const fn opt_tagged(self, tag: Option<Tag>) -> Asn {
        Asn::opt_tagged(tag, self)
    }

    pub const fn tagged(self, tag: Tag) -> Asn {
        Asn::tagged(tag, self)
    }

    pub const fn untagged(self) -> Asn {
        Asn::untagged(self)
    }

    pub fn no_optional_mut(&mut self) -> &mut Self {
        if let Self::Optional(inner) = self {
            inner.no_optional_mut()
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Integer {
    pub range: Range<Option<i64>>,
    pub constants: Vec<(String, i64)>,
}

impl TryFrom<&mut Peekable<IntoIter<Token>>> for Integer {
    type Error = Error;

    fn try_from(iter: &mut Peekable<IntoIter<Token>>) -> Result<Self, Self::Error> {
        let constants =
            Model::<Asn>::maybe_read_constants(iter, Model::<Asn>::constant_i64_parser)?;
        let range = Model::<Asn>::read_number_range(iter)?;
        Ok(Self { range, constants })
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct BitString {
    pub size: Size,
    pub constants: Vec<(String, u64)>,
}

impl TryFrom<&mut Peekable<IntoIter<Token>>> for BitString {
    type Error = Error;

    fn try_from(iter: &mut Peekable<IntoIter<Token>>) -> Result<Self, Self::Error> {
        let constants =
            Model::<Asn>::maybe_read_constants(iter, Model::<Asn>::constant_u64_parser)?;
        let size = Model::<Asn>::maybe_read_size(iter)?;
        Ok(Self { size, constants })
    }
}

/// ITU-T X.680 | ISO/IEC 8824-1:2015, Annex L
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ComponentTypeList {
    pub fields: Vec<Field<Asn>>,
    pub extension_after: Option<usize>,
}

impl TryFrom<&mut Peekable<IntoIter<Token>>> for ComponentTypeList {
    type Error = Error;

    fn try_from(iter: &mut Peekable<IntoIter<Token>>) -> Result<Self, Self::Error> {
        Model::<Asn>::next_separator_ignore_case(iter, '{')?;
        let mut sequence = Self {
            fields: Vec::default(),
            extension_after: None,
        };

        loop {
            let continues = if Model::<Asn>::peek(iter)?.eq_separator('.') {
                let _ = Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                let _ = Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                let _ = Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                let field_len = sequence.fields.len();
                sequence.extension_after = Some(field_len.saturating_sub(1));
                let token = Model::<Asn>::next(iter)?;
                if token.eq_separator(',') {
                    true
                } else if token.eq_separator('}') {
                    false
                } else {
                    return Err(Error::unexpected_token(token));
                }
            } else {
                let (field, continues) = Model::<Asn>::read_field(iter)?;
                sequence.fields.push(field);
                continues
            };

            if !continues {
                break;
            }
        }

        Ok(sequence)
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Choice {
    variants: Vec<ChoiceVariant>,
    extension_after: Option<usize>,
}

impl From<Vec<ChoiceVariant>> for Choice {
    fn from(variants: Vec<ChoiceVariant>) -> Self {
        Self {
            variants,
            extension_after: None,
        }
    }
}

impl Choice {
    pub fn from_variants(variants: impl Iterator<Item = ChoiceVariant>) -> Self {
        Self {
            variants: variants.collect(),
            extension_after: None,
        }
    }

    pub const fn with_extension_after(mut self, extension_after: Option<usize>) -> Self {
        self.extension_after = extension_after;
        self
    }

    pub fn len(&self) -> usize {
        self.variants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    pub fn variants(&self) -> impl Iterator<Item = &ChoiceVariant> {
        self.variants.iter()
    }

    pub fn is_extensible(&self) -> bool {
        self.extension_after.is_some()
    }

    pub fn extension_after_index(&self) -> Option<usize> {
        self.extension_after
    }
}

impl TryFrom<&mut Peekable<IntoIter<Token>>> for Choice {
    type Error = Error;

    fn try_from(iter: &mut Peekable<IntoIter<Token>>) -> Result<Self, Self::Error> {
        Model::<Asn>::next_separator_ignore_case(iter, '{')?;
        let mut choice = Choice {
            variants: Vec::new(),
            extension_after: None,
        };

        loop {
            let name_or_extension_marker = Model::<Asn>::next(iter)?;
            if name_or_extension_marker.eq_separator('.') {
                Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                Model::<Asn>::next_separator_ignore_case(iter, '.')?;

                if choice.variants.is_empty() || choice.extension_after.is_some() {
                    return Err(Error::invalid_position_for_extension_marker(
                        name_or_extension_marker,
                    ));
                } else {
                    choice.extension_after = Some(choice.variants.len() - 1);
                }
            } else {
                let name = name_or_extension_marker.into_text_or_else(Error::no_text)?;
                let (token, tag) = Model::<Asn>::next_with_opt_tag(iter)?;
                let r#type = Model::<Asn>::read_role_given_text(
                    iter,
                    token.into_text_or_else(Error::no_text)?,
                )?;
                choice.variants.push(ChoiceVariant { name, tag, r#type });
            }

            let end_or_continuation_marker = Model::<Asn>::next(iter)?;

            if end_or_continuation_marker.eq_separator(',') {
                continue;
            } else if end_or_continuation_marker.eq_separator('}') {
                break;
            } else {
                return Err(Error::unexpected_token(end_or_continuation_marker));
            }
        }

        Ok(choice)
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ChoiceVariant {
    pub name: String,
    pub tag: Option<Tag>,
    pub r#type: Type,
}

impl ChoiceVariant {
    #[cfg(test)]
    pub fn name_type<I: ToString>(name: I, r#type: Type) -> Self {
        ChoiceVariant {
            name: name.to_string(),
            tag: None,
            r#type,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn r#type(&self) -> &Type {
        &self.r#type
    }
}

impl TagProperty for ChoiceVariant {
    fn tag(&self) -> Option<Tag> {
        self.tag
    }

    fn set_tag(&mut self, tag: Tag) {
        self.tag = Some(tag)
    }

    fn reset_tag(&mut self) {
        self.tag = None
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Enumerated {
    variants: Vec<EnumeratedVariant>,
    extension_after: Option<usize>,
}

impl From<Vec<EnumeratedVariant>> for Enumerated {
    fn from(variants: Vec<EnumeratedVariant>) -> Self {
        Self {
            variants,
            extension_after: None,
        }
    }
}

impl Enumerated {
    pub fn from_variants(variants: impl Into<Vec<EnumeratedVariant>>) -> Self {
        Self {
            variants: variants.into(),
            extension_after: None,
        }
    }

    pub fn from_names<I: ToString>(variants: impl Iterator<Item = I>) -> Self {
        Self {
            variants: variants.map(EnumeratedVariant::from_name).collect(),
            extension_after: None,
        }
    }

    pub const fn with_extension_after(mut self, extension_after: Option<usize>) -> Self {
        self.extension_after = extension_after;
        self
    }

    pub fn len(&self) -> usize {
        self.variants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    pub fn variants(&self) -> impl Iterator<Item = &EnumeratedVariant> {
        self.variants.iter()
    }

    pub fn is_extensible(&self) -> bool {
        self.extension_after.is_some()
    }

    pub fn extension_after_index(&self) -> Option<usize> {
        self.extension_after
    }
}

impl TryFrom<&mut Peekable<IntoIter<Token>>> for Enumerated {
    type Error = Error;

    fn try_from(iter: &mut Peekable<IntoIter<Token>>) -> Result<Self, Self::Error> {
        Model::<Asn>::next_separator_ignore_case(iter, '{')?;
        let mut enumerated = Self {
            variants: Vec::new(),
            extension_after: None,
        };

        loop {
            let token = Model::<Asn>::next(iter)?;

            if token.eq_separator('.') {
                Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                if enumerated.variants.is_empty() || enumerated.extension_after.is_some() {
                    return Err(Error::invalid_position_for_extension_marker(token));
                } else {
                    enumerated.extension_after = Some(enumerated.variants.len() - 1);
                    loop_ctrl_separator!(Model::<Asn>::next(iter)?);
                }
            } else {
                let variant_name = token.into_text_or_else(Error::no_text)?;
                let token = Model::<Asn>::next(iter)?;

                if token.eq_separator(',') || token.eq_separator('}') {
                    enumerated
                        .variants
                        .push(EnumeratedVariant::from_name(variant_name));
                    loop_ctrl_separator!(token);
                } else if token.eq_separator('(') {
                    let token = Model::<Asn>::next(iter)?;
                    let number = token
                        .text()
                        .and_then(|t| t.parse::<usize>().ok())
                        .ok_or_else(|| Error::invalid_number_for_enum_variant(token))?;
                    Model::<Asn>::next_separator_ignore_case(iter, ')')?;
                    enumerated
                        .variants
                        .push(EnumeratedVariant::from_name_number(variant_name, number));
                    loop_ctrl_separator!(Model::<Asn>::next(iter)?);
                } else {
                    loop_ctrl_separator!(token);
                }
            }
        }

        Ok(enumerated)
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct EnumeratedVariant {
    pub(crate) name: String,
    pub(crate) number: Option<usize>,
}

#[cfg(test)]
impl<S: ToString> From<S> for EnumeratedVariant {
    fn from(s: S) -> Self {
        EnumeratedVariant::from_name(s)
    }
}

impl EnumeratedVariant {
    pub fn from_name<I: ToString>(name: I) -> Self {
        Self {
            name: name.to_string(),
            number: None,
        }
    }

    pub fn from_name_number<I: ToString>(name: I, number: usize) -> Self {
        Self {
            name: name.to_string(),
            number: Some(number),
        }
    }

    pub const fn with_number(self, number: usize) -> Self {
        self.with_number_opt(Some(number))
    }

    pub const fn with_number_opt(mut self, number: Option<usize>) -> Self {
        self.number = number;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn number(&self) -> Option<usize> {
        self.number
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::parser::{Location, Tokenizer};

    pub(crate) const SIMPLE_INTEGER_STRUCT_ASN: &str = r"
        SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
        BEGIN

        Simple ::= SEQUENCE {
            small INTEGER(0..255),
            bigger INTEGER(0..65535),
            negative INTEGER(-1..255),
            unlimited INTEGER(0..MAX) OPTIONAL
        }
        END
        ";

    #[test]
    fn test_simple_asn_sequence_represented_correctly_as_asn_model() {
        let model = Model::try_from(Tokenizer::default().parse(SIMPLE_INTEGER_STRUCT_ASN)).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(1, model.definitions.len());
        assert_eq!(
            Definition(
                "Simple".into(),
                Type::sequence_from_fields(vec![
                    Field {
                        name: "small".into(),
                        role: Type::integer_with_range(Range::inclusive(Some(0), Some(255)))
                            .untagged(),
                    },
                    Field {
                        name: "bigger".into(),
                        role: Type::integer_with_range(Range::inclusive(Some(0), Some(65535)))
                            .untagged(),
                    },
                    Field {
                        name: "negative".into(),
                        role: Type::integer_with_range(Range::inclusive(Some(-1), Some(255)))
                            .untagged(),
                    },
                    Field {
                        name: "unlimited".into(),
                        role: Type::unconstrained_integer().optional().untagged(),
                    }
                ])
                .untagged(),
            ),
            model.definitions[0]
        );
    }

    pub(crate) const INLINE_ASN_WITH_ENUM: &str = r"
        SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
        BEGIN

        Woah ::= SEQUENCE {
            decision ENUMERATED {
                ABORT,
                RETURN,
                CONFIRM,
                MAYDAY,
                THE_CAKE_IS_A_LIE
            } OPTIONAL
        }
        END
    ";

    #[test]
    fn test_inline_asn_enumerated_represented_correctly_as_asn_model() {
        let model = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_ENUM)).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(1, model.definitions.len());
        assert_eq!(
            Definition(
                "Woah".into(),
                Type::sequence_from_fields(vec![Field {
                    name: "decision".into(),
                    role: Type::Enumerated(Enumerated::from_names(
                        ["ABORT", "RETURN", "CONFIRM", "MAYDAY", "THE_CAKE_IS_A_LIE",].iter()
                    ))
                    .optional()
                    .untagged(),
                }])
                .untagged(),
            ),
            model.definitions[0]
        );
    }

    pub(crate) const INLINE_ASN_WITH_SEQUENCE_OF: &str = r"
        SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
        BEGIN

        Ones ::= SEQUENCE OF INTEGER(0..1)

        NestedOnes ::= SEQUENCE OF SEQUENCE OF INTEGER(0..1)

        Woah ::= SEQUENCE {
            also-ones SEQUENCE OF INTEGER(0..1),
            nesteds SEQUENCE OF SEQUENCE OF INTEGER(0..1),
            optionals SEQUENCE OF SEQUENCE OF INTEGER(0..MAX) OPTIONAL
        }
        END
    ";

    #[test]
    fn test_inline_asn_sequence_of_represented_correctly_as_asn_model() {
        let model =
            Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_SEQUENCE_OF)).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(3, model.definitions.len());
        assert_eq!(
            Definition(
                "Ones".into(),
                Type::SequenceOf(
                    Box::new(Type::integer_with_range(Range::inclusive(Some(0), Some(1)))),
                    Size::Any,
                )
                .untagged(),
            ),
            model.definitions[0]
        );
        assert_eq!(
            Definition(
                "NestedOnes".into(),
                Type::SequenceOf(
                    Box::new(Type::SequenceOf(
                        Box::new(Type::integer_with_range(Range::inclusive(Some(0), Some(1)))),
                        Size::Any,
                    )),
                    Size::Any,
                )
                .untagged(),
            ),
            model.definitions[1]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Type::sequence_from_fields(vec![
                    Field {
                        name: "also-ones".into(),
                        role: Type::SequenceOf(
                            Box::new(Type::integer_with_range(Range::inclusive(Some(0), Some(1)))),
                            Size::Any,
                        )
                        .untagged(),
                    },
                    Field {
                        name: "nesteds".into(),
                        role: Type::SequenceOf(
                            Box::new(Type::SequenceOf(
                                Box::new(Type::integer_with_range(Range::inclusive(
                                    Some(0),
                                    Some(1),
                                ))),
                                Size::Any,
                            )),
                            Size::Any,
                        )
                        .untagged(),
                    },
                    Field {
                        name: "optionals".into(),
                        role: Type::SequenceOf(
                            Box::new(Type::SequenceOf(
                                Box::new(Type::unconstrained_integer()),
                                Size::Any,
                            )),
                            Size::Any,
                        )
                        .optional()
                        .untagged(),
                    },
                ])
                .untagged(),
            ),
            model.definitions[2]
        );
    }

    pub(crate) const INLINE_ASN_WITH_CHOICE: &str = r"
        SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
        BEGIN

        This ::= SEQUENCE OF INTEGER(0..1)

        That ::= SEQUENCE OF SEQUENCE OF INTEGER(0..1)

        Neither ::= ENUMERATED {
            ABC,
            DEF
        }

        Woah ::= SEQUENCE {
            decision CHOICE {
                this This,
                that That,
                neither Neither
            }
        }
        END
    ";

    #[test]
    fn test_inline_asn_choice_represented_correctly_as_asn_model() {
        let model = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_CHOICE)).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(4, model.definitions.len());
        assert_eq!(
            Definition(
                "This".into(),
                Type::SequenceOf(
                    Box::new(Type::integer_with_range(Range::inclusive(Some(0), Some(1)))),
                    Size::Any,
                )
                .untagged(),
            ),
            model.definitions[0]
        );
        assert_eq!(
            Definition(
                "That".into(),
                Type::SequenceOf(
                    Box::new(Type::SequenceOf(
                        Box::new(Type::integer_with_range(Range::inclusive(Some(0), Some(1)))),
                        Size::Any,
                    )),
                    Size::Any,
                )
                .untagged(),
            ),
            model.definitions[1]
        );
        assert_eq!(
            Definition(
                "Neither".into(),
                Type::Enumerated(Enumerated::from_names(["ABC", "DEF"].iter())).untagged(),
            ),
            model.definitions[2]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Type::sequence_from_fields(vec![Field {
                    name: "decision".into(),
                    role: Type::choice_from_variants(vec![
                        ChoiceVariant::name_type("this", Type::TypeReference("This".into(), None)),
                        ChoiceVariant::name_type("that", Type::TypeReference("That".into(), None)),
                        ChoiceVariant::name_type(
                            "neither",
                            Type::TypeReference("Neither".into(), None)
                        ),
                    ])
                    .untagged(),
                }])
                .untagged(),
            ),
            model.definitions[3]
        );
    }

    pub(crate) const INLINE_ASN_WITH_SEQUENCE: &str = r"
        SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
        BEGIN

        Woah ::= SEQUENCE {
            complex SEQUENCE {
                ones INTEGER(0..1),
                list-ones SEQUENCE OF INTEGER(0..1),
                optional-ones SEQUENCE OF INTEGER(0..1) OPTIONAL
            } OPTIONAL
        }
        END
    ";

    #[test]
    fn test_inline_asn_sequence_represented_correctly_as_asn_model() {
        let model = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_SEQUENCE)).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(1, model.definitions.len());
        assert_eq!(
            Definition(
                "Woah".into(),
                Type::sequence_from_fields(vec![Field {
                    name: "complex".into(),
                    role: Type::sequence_from_fields(vec![
                        Field {
                            name: "ones".into(),
                            role: Type::integer_with_range(Range::inclusive(Some(0), Some(1)))
                                .untagged(),
                        },
                        Field {
                            name: "list-ones".into(),
                            role: Type::SequenceOf(
                                Box::new(Type::integer_with_range(Range::inclusive(
                                    Some(0),
                                    Some(1),
                                ))),
                                Size::Any,
                            )
                            .untagged(),
                        },
                        Field {
                            name: "optional-ones".into(),
                            role: Type::SequenceOf(
                                Box::new(Type::integer_with_range(Range::inclusive(
                                    Some(0),
                                    Some(1),
                                ))),
                                Size::Any,
                            )
                            .optional()
                            .untagged(),
                        },
                    ])
                    .optional()
                    .untagged(),
                }])
                .untagged(),
            ),
            model.definitions[0]
        );
    }

    #[test]
    fn test_nice_names() {
        let mut model = Model::default();

        model.name = "SimpleTest".into();
        model.make_names_nice();
        assert_eq!("simple_test", model.to_rust().name);

        model.name = "SIMPLE_Test".into();
        model.make_names_nice();
        assert_eq!("simple_test", model.to_rust().name);

        model.name = "DRY_Module".into();
        model.make_names_nice();
        assert_eq!("dry", model.to_rust().name);

        model.name = "DRYModule".into();
        model.make_names_nice();
        assert_eq!("dry", model.to_rust().name);
    }

    #[test]
    pub fn test_integer_type_with_range() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"
            SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
            BEGIN
    
            SimpleTypeWithRange ::= Integer (0..65535)
            
            END
        ",
        ))
        .expect("Failed to parse");

        assert_eq!("SimpleSchema", &model.name);
        assert_eq!(
            &[Definition(
                "SimpleTypeWithRange".to_string(),
                Type::integer_with_range(Range::inclusive(Some(0), Some(65_535))).untagged(),
            )][..],
            &model.definitions[..]
        )
    }

    #[test]
    pub fn test_string_type() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"
            SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
            BEGIN
    
            SimpleStringType ::= UTF8String
            
            END
        ",
        ))
        .expect("Failed to parse");

        assert_eq!("SimpleSchema", &model.name);
        assert_eq!(
            &[Definition(
                "SimpleStringType".to_string(),
                Type::unconstrained_utf8string().untagged(),
            )][..],
            &model.definitions[..]
        )
    }

    #[test]
    pub fn test_enumerated_advanced() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
            BEGIN
    
            Basic ::= ENUMERATED {
                abc,
                def
            }
    
            WithExplicitNumber ::= ENUMERATED {
                abc(1),
                def(9)
            }
            
            WithExplicitNumberAndDefaultMark ::= ENUMERATED {
                abc(4),
                def(7),
                ...
            }
            
            WithExplicitNumberAndDefaultMarkV2 ::= ENUMERATED {
                abc(8),
                def(1),
                ...,
                v2(11)
            }
            
            END
        ",
        ))
        .expect("Failed to parse");

        assert_eq!("SimpleSchema", &model.name);
        assert_eq!(
            &[
                Definition(
                    "Basic".to_string(),
                    Type::Enumerated(Enumerated::from_names(["abc", "def"].iter())).untagged(),
                ),
                Definition(
                    "WithExplicitNumber".to_string(),
                    Type::Enumerated(Enumerated {
                        variants: vec![
                            EnumeratedVariant::from_name_number("abc", 1),
                            EnumeratedVariant::from_name_number("def", 9)
                        ],
                        extension_after: None,
                    })
                    .untagged(),
                ),
                Definition(
                    "WithExplicitNumberAndDefaultMark".to_string(),
                    Type::Enumerated(Enumerated {
                        variants: vec![
                            EnumeratedVariant::from_name_number("abc", 4),
                            EnumeratedVariant::from_name_number("def", 7),
                        ],
                        extension_after: Some(1),
                    })
                    .untagged(),
                ),
                Definition(
                    "WithExplicitNumberAndDefaultMarkV2".to_string(),
                    Type::Enumerated(Enumerated {
                        variants: vec![
                            EnumeratedVariant::from_name_number("abc", 8),
                            EnumeratedVariant::from_name_number("def", 1),
                            EnumeratedVariant::from_name_number("v2", 11)
                        ],
                        extension_after: Some(1),
                    })
                    .untagged(),
                )
            ][..],
            &model.definitions[..]
        )
    }

    #[test]
    pub fn test_enumerated_tags() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
            BEGIN
    
            Universal ::= [UNIVERSAL 2] ENUMERATED {
                abc,
                def
            }
    
            Application ::= [APPLICATION 7] ENUMERATED {
                abc,
                def
            }
            
            Private ::= [PRIVATE 11] ENUMERATED {
                abc,
                def
            }
            
            ContextSpecific ::= [8] ENUMERATED {
                abc,
                def
            }
            
            END
        ",
        ))
        .expect("Failed to parse");

        assert_eq!("SimpleSchema", &model.name);
        assert_eq!(
            &[
                Definition(
                    "Universal".to_string(),
                    Type::Enumerated(Enumerated::from_names(["abc", "def"].iter()))
                        .tagged(Tag::Universal(2)),
                ),
                Definition(
                    "Application".to_string(),
                    Type::Enumerated(Enumerated::from_names(["abc", "def"].iter()))
                        .tagged(Tag::Application(7)),
                ),
                Definition(
                    "Private".to_string(),
                    Type::Enumerated(Enumerated::from_names(["abc", "def"].iter()))
                        .tagged(Tag::Private(11)),
                ),
                Definition(
                    "ContextSpecific".to_string(),
                    Type::Enumerated(Enumerated::from_names(["abc", "def"].iter()))
                        .tagged(Tag::ContextSpecific(8)),
                ),
            ][..],
            &model.definitions[..]
        )
    }

    #[test]
    pub fn test_parsing_tags_in_front_of_definitions_does_not_fail() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
            BEGIN
    
            Universal ::= [UNIVERSAL 2] SEQUENCE {
                abc [1] INTEGER(0..MAX),
                def [2] INTEGER(0..255)
            }
    
            Application ::= [APPLICATION 7] SEQUENCE OF UTF8String
            
            Private ::= [PRIVATE 11] ENUMERATED {
                abc,
                def
            }
            
            ContextSpecific ::= [8] INTEGER(0..MAX)
            
            END
        ",
        ))
        .expect("Failed to parse");

        assert_eq!("SimpleSchema", &model.name);
        assert_eq!(
            &[
                Definition(
                    "Universal".to_string(),
                    Type::sequence_from_fields(vec![
                        Field {
                            name: "abc".to_string(),
                            role: Type::unconstrained_integer().tagged(Tag::ContextSpecific(1)),
                        },
                        Field {
                            name: "def".to_string(),
                            role: Type::integer_with_range(Range::inclusive(Some(0), Some(255)))
                                .tagged(Tag::ContextSpecific(2)),
                        }
                    ])
                    .tagged(Tag::Universal(2)),
                ),
                Definition(
                    "Application".to_string(),
                    Type::SequenceOf(Box::new(Type::unconstrained_utf8string()), Size::Any)
                        .tagged(Tag::Application(7)),
                ),
                Definition(
                    "Private".to_string(),
                    Type::Enumerated(Enumerated::from_names(["abc", "def"].iter()))
                        .tagged(Tag::Private(11)),
                ),
                Definition(
                    "ContextSpecific".to_string(),
                    Type::unconstrained_integer().tagged(Tag::ContextSpecific(8)),
                ),
            ][..],
            &model.definitions[..]
        )
    }

    #[test]
    pub fn test_parsing_of_extensible_choices() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
            BEGIN
    
            WithoutMarker ::= CHOICE {
                abc UTF8String,
                def UTF8String
            }
            
            WithoutExtensionPresent ::= CHOICE {
                abc UTF8String,
                def UTF8String,
                ...
            }
    
            WithExtensionPresent ::= CHOICE {
                abc UTF8String,
                def UTF8String,
                ...,
                ghi UTF8String
            }
            
            END
        ",
        ))
        .expect("Failed to parse");

        assert_eq!("SimpleSchema", model.name.as_str());
        assert_eq!(
            &[
                Definition::new(
                    "WithoutMarker",
                    Type::Choice(Choice {
                        variants: vec![
                            ChoiceVariant::name_type("abc", Type::unconstrained_utf8string()),
                            ChoiceVariant::name_type("def", Type::unconstrained_utf8string()),
                        ],
                        extension_after: None,
                    })
                    .untagged(),
                ),
                Definition::new(
                    "WithoutExtensionPresent",
                    Type::Choice(Choice {
                        variants: vec![
                            ChoiceVariant::name_type("abc", Type::unconstrained_utf8string()),
                            ChoiceVariant::name_type("def", Type::unconstrained_utf8string()),
                        ],
                        extension_after: Some(1),
                    })
                    .untagged(),
                ),
                Definition::new(
                    "WithExtensionPresent",
                    Type::Choice(Choice {
                        variants: vec![
                            ChoiceVariant::name_type("abc", Type::unconstrained_utf8string()),
                            ChoiceVariant::name_type("def", Type::unconstrained_utf8string()),
                            ChoiceVariant::name_type("ghi", Type::unconstrained_utf8string()),
                        ],
                        extension_after: Some(1),
                    })
                    .untagged(),
                )
            ][..],
            &model.definitions[..]
        )
    }

    #[test]
    pub fn test_parsing_of_extensible_with_markers_at_invalid_locations() {
        assert_eq!(
            Error::invalid_position_for_extension_marker(Token::Separator(
                Location::at(4, 21),
                '.',
            )),
            Model::try_from(Tokenizer::default().parse(
                r"SimpleSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

                Invalid ::= CHOICE {
                    ...
                }
                
                END",
            ))
            .expect_err("Parsed invalid definition")
        );

        assert_eq!(
            Error::invalid_position_for_extension_marker(Token::Separator(
                Location::at(4, 21),
                '.',
            )),
            Model::try_from(Tokenizer::default().parse(
                r"SimpleSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN
    
                Invalid ::= CHOICE {
                    ...,
                    abc UTF8String
                }
                
                END",
            ))
            .expect_err("Parsed invalid definition")
        );

        assert_eq!(
            Error::invalid_position_for_extension_marker(Token::Separator(
                Location::at(4, 21),
                '.',
            )),
            Model::try_from(Tokenizer::default().parse(
                r"SimpleSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN
    
                Invalid ::= ENUMERATED {
                    ...
                }
                
                END",
            ))
            .expect_err("Parsed invalid definition")
        );

        assert_eq!(
            Error::invalid_position_for_extension_marker(Token::Separator(
                Location::at(4, 21),
                '.',
            )),
            Model::try_from(Tokenizer::default().parse(
                r"SimpleSchema DEFINITIONS AUTOMATIC TAGS ::= BEGIN

                Invalid ::= ENUMERATED {
                    ...,
                    abc(77)
                }
                
                END",
            ))
            .expect_err("Parsed invalid definition")
        );
    }

    #[test]
    pub fn test_parsing_module_definition_oid() {
        let model = Model::try_from(Tokenizer::default().parse(
            "SomeName { very(1) clever oid(4) 1337 } DEFINITIONS AUTOMATIC TAGS ::= BEGIN END",
        ))
        .expect("Failed to load model");
        assert_eq!(
            ObjectIdentifier(vec![
                ObjectIdentifierComponent::NameAndNumberForm("very".to_string(), 1),
                ObjectIdentifierComponent::NameForm("clever".to_string()),
                ObjectIdentifierComponent::NameAndNumberForm("oid".to_string(), 4),
                ObjectIdentifierComponent::NumberForm(1337),
            ]),
            model.oid.expect("ObjectIdentifier is missing")
        )
    }

    #[test]
    pub fn test_parsing_module_definition_oid_in_import_from() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN
                IMPORTS
                    SomeData, OtherDef, Wowz
                FROM TheOtherModule { very(1) official(2) oid 42 };
                END",
        ))
        .expect("Failed to load model");
        assert_eq!(
            &ObjectIdentifier(vec![
                ObjectIdentifierComponent::NameAndNumberForm("very".to_string(), 1),
                ObjectIdentifierComponent::NameAndNumberForm("official".to_string(), 2),
                ObjectIdentifierComponent::NameForm("oid".to_string()),
                ObjectIdentifierComponent::NumberForm(42),
            ]),
            model.imports[0]
                .from_oid
                .as_ref()
                .expect("ObjectIdentifier is missing")
        )
    }

    #[test]
    pub fn test_parsing_module_definition_with_integer_constant() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN
                TheGreatStruct ::= SEQUENCE {
                    inline     INTEGER { ab(1), cd(2), ef(3) },
                    eff-u8     INTEGER { gh(1), ij(4), kl(9) } (0..255),
                    tagged [7] INTEGER { mn(5), op(4), qr(9) } (0..255) 
                }
                
                SeAlias ::= INTEGER { wow(1), much(2), great(3) }
                
                OhAlias ::= [APPLICATION 9] INTEGER { oh(1), lul(2) } (0..255)
                END",
        ))
        .expect("Failed to load model");
        assert_eq!(
            vec![
                Definition(
                    "TheGreatStruct".to_string(),
                    Type::sequence_from_fields(vec![
                        Field {
                            name: "inline".to_string(),
                            role: Type::Integer(Integer {
                                range: Range::none(),
                                constants: vec![
                                    ("ab".to_string(), 1),
                                    ("cd".to_string(), 2),
                                    ("ef".to_string(), 3)
                                ],
                            })
                            .untagged(),
                        },
                        Field {
                            name: "eff-u8".to_string(),
                            role: Type::Integer(Integer {
                                range: Range::inclusive(Some(0), Some(255)),
                                constants: vec![
                                    ("gh".to_string(), 1),
                                    ("ij".to_string(), 4),
                                    ("kl".to_string(), 9)
                                ],
                            })
                            .untagged(),
                        },
                        Field {
                            name: "tagged".to_string(),
                            role: Type::Integer(Integer {
                                range: Range::inclusive(Some(0), Some(255)),
                                constants: vec![
                                    ("mn".to_string(), 5),
                                    ("op".to_string(), 4),
                                    ("qr".to_string(), 9)
                                ],
                            })
                            .tagged(Tag::ContextSpecific(7)),
                        },
                    ])
                    .untagged(),
                ),
                Definition(
                    "SeAlias".to_string(),
                    Type::Integer(Integer {
                        range: Range::none(),
                        constants: vec![
                            ("wow".to_string(), 1),
                            ("much".to_string(), 2),
                            ("great".to_string(), 3),
                        ],
                    })
                    .untagged(),
                ),
                Definition(
                    "OhAlias".to_string(),
                    Type::Integer(Integer {
                        range: Range::inclusive(Some(0), Some(255)),
                        constants: vec![("oh".to_string(), 1), ("lul".to_string(), 2),],
                    })
                    .tagged(Tag::Application(9)),
                )
            ],
            model.definitions
        )
    }

    #[test]
    pub fn test_parsing_module_definition_with_extensible_integer() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN
                RangedOptional ::= SEQUENCE {
                    value     INTEGER { gh(1), ij(4), kl(9) } (0..255,...) OPTIONAL
                }
                
                END",
        ))
        .expect("Failed to load model");
        assert_eq!(
            vec![Definition(
                "RangedOptional".to_string(),
                Type::sequence_from_fields(vec![Field {
                    name: "value".to_string(),
                    role: Type::Integer(Integer {
                        range: Range::inclusive(Some(0), Some(255)).with_extensible(true),
                        constants: vec![
                            ("gh".to_string(), 1),
                            ("ij".to_string(), 4),
                            ("kl".to_string(), 9)
                        ],
                    })
                    .optional()
                    .untagged(),
                }])
                .untagged(),
            )],
            model.definitions
        )
    }

    #[test]
    pub fn test_resolve_tag() {
        let external = Model::try_from(Tokenizer::default().parse(
            r"ExternalModule DEFINITIONS AUTOMATIC TAGS ::= BEGIN
            External ::= [APPLICATION 1] INTEGER
            END
            ",
        ))
        .expect("Failed to parse module");
        let model = Model::try_from(Tokenizer::default().parse(
            r"InternalModul DEFINITIONS AUTOMATIC TAGS ::= BEGIN
                IMPORTS
                    External
                FROM ExternalModule;
                
                Implicit ::= SEQUENCE {
                    implicit     INTEGER OPTIONAL,
                    explicit [4] INTEGER 
                }
                
                Explicit ::= [APPLICATION 8] ENUMERATED {
                    abc,
                    def
                }
                
                Composed ::= CHOICE {
                    first-but-greater-tag-value [APPLICATION 99] INTEGER,
                    second-but-indirect-lower-tag Explicit
                }
                
                ExternallyComposed ::= CHOICE {
                    internal Explicit,
                    extenral External
                }
                
                END",
        ))
        .expect("Failed to load model");
        let rust = model.to_rust_with_scope(&[&external]);

        if let Rust::Struct {
            ordering: _,
            fields,
            tag,
            extension_after: _,
        } = rust.definitions[0].value()
        {
            assert_eq!("Implicit", rust.definitions[0].0.as_str());
            assert_eq!(None, *tag); // None because default
            assert_eq!(None, fields[0].tag()); // None because default
            assert_eq!(Some(Tag::ContextSpecific(4)), fields[1].tag()); // explicitly set
        } else {
            panic!("Expected Rust::Struct for ASN.1 SEQUENCE");
        }

        if let Rust::Enum(plain) = rust.definitions[1].value() {
            assert_eq!("Explicit", rust.definitions[1].0.as_str());
            assert_eq!(2, plain.len());
            assert_eq!(Some(Tag::Application(8)), plain.tag()); // explicitly set
        } else {
            panic!("Expected Rust::Enum for ASN.1 ENUMERATED")
        }

        if let Rust::DataEnum(data) = rust.definitions[2].value() {
            assert_eq!("Composed", rust.definitions[2].0.as_str());
            assert_eq!(2, data.len());
            assert_eq!(None, data.tag()); // None because no tag explicitly set
        } else {
            panic!("Expected Rust::DataEnum for ASN.1 CHOICE")
        }

        if let Rust::DataEnum(data) = rust.definitions[3].value() {
            assert_eq!("ExternallyComposed", rust.definitions[3].0.as_str());
            assert_eq!(2, data.len());
            assert_eq!(None, data.tag()); // None because no tag explicitly set
        } else {
            panic!("Expected Rust::DataEnum for ASN.1 CHOICE")
        }

        assert_eq!(4, rust.definitions.len());
    }
}
