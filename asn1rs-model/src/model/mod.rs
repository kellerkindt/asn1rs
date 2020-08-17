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
            match iter.peek() {
                Some(peeked) if peeked.eq_separator('(') => {
                    Self::next_separator_ignore_case(iter, '(')?;
                    let start = Self::next(iter)?;
                    Self::next_separator_ignore_case(iter, '.')?;
                    Self::next_separator_ignore_case(iter, '.')?;
                    let end = Self::next(iter)?;
                    Self::next_separator_ignore_case(iter, ')')?;
                    if start.eq_text("0") && end.eq_text_ignore_ascii_case("MAX") {
                        Ok(Type::Integer(None))
                    } else {
                        Ok(Type::Integer(Some(Range(
                            start
                                .text()
                                .and_then(|t| t.parse::<i64>().ok())
                                .ok_or_else(|| Error::invalid_range_value(start))?,
                            end.text()
                                .and_then(|t| t.parse::<i64>().ok())
                                .ok_or_else(|| Error::invalid_range_value(end))?,
                        ))))
                    }
                }
                _ => Ok(Type::Integer(None)),
            }
        } else if text.eq_ignore_ascii_case("BOOLEAN") {
            Ok(Type::Boolean)
        } else if text.eq_ignore_ascii_case("UTF8String") {
            Ok(Type::UTF8String)
        } else if text.eq_ignore_ascii_case("OCTET") {
            let token = Self::next(iter)?;
            if token.text().map_or(false, |t| t.eq("STRING")) {
                Ok(Type::OctetString)
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
            Ok(Type::TypeReference(text))
        }
    }

    fn read_sequence_or_sequence_of(iter: &mut Peekable<IntoIter<Token>>) -> Result<Type, Error> {
        let token = Self::peek(iter)?;

        if token.eq_text_ignore_ascii_case("OF") {
            let _ = Self::next(iter)?;
            Ok(Type::SequenceOf(Box::new(Self::read_role(iter)?)))
        } else if token.eq_separator('{') {
            Ok(Type::Sequence(Sequence::try_from(iter)?))
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
        Model::convert_asn_to_rust(self)
    }
}

#[derive(Debug, Default, Clone, PartialOrd, PartialEq)]
pub struct Import {
    pub what: Vec<String>,
    pub from: String,
    pub from_oid: Option<ObjectIdentifier>,
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub struct Range<T>(pub T, pub T);

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Definition<T>(pub String, pub T);

impl<T> Definition<T> {
    #[cfg(test)]
    pub fn new<I: ToString>(name: I, value: T) -> Self {
        Definition(name.to_string(), value)
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
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq)]
pub enum Tag {
    Universal(usize),
    Application(usize),
    ContextSpecific(usize),
    Private(usize),
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

pub trait TagProperty {
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

    pub fn extensible_after_index(&self) -> Option<usize> {
        match &self.r#type {
            Type::Choice(c) => c.extension_after_index(),
            Type::Enumerated(e) => e.extension_after_index(),
            _ => None,
        }
    }

    pub fn extensible_after_variant(&self) -> Option<&str> {
        match &self.r#type {
            Type::Choice(c) => c
                .extension_after_index()
                .and_then(|index| c.variants().nth(index).map(ChoiceVariant::name)),
            Type::Enumerated(e) => e
                .extension_after_index()
                .and_then(|index| e.variants().nth(index).map(EnumeratedVariant::name)),
            _ => None,
        }
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
    Integer(Option<Range<i64>>),
    UTF8String,
    OctetString,

    Optional(Box<Type>),

    Sequence(Sequence),
    SequenceOf(Box<Type>),
    Enumerated(Enumerated),
    Choice(Choice),
    TypeReference(String),
}

impl Type {
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
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Sequence {
    fields: Vec<Field<Asn>>,
}

impl From<Vec<Field<Asn>>> for Sequence {
    fn from(fields: Vec<Field<Asn>>) -> Self {
        Self { fields }
    }
}

impl TryFrom<&mut Peekable<IntoIter<Token>>> for Sequence {
    type Error = Error;

    fn try_from(iter: &mut Peekable<IntoIter<Token>>) -> Result<Self, Self::Error> {
        Model::<Asn>::next_separator_ignore_case(iter, '{')?;
        let mut fields = Vec::new();

        loop {
            let (field, continues) = Model::<Asn>::read_field(iter)?;
            fields.push(field);
            if !continues {
                break;
            }
        }

        Ok(Self { fields })
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
                Type::Sequence(Sequence::from(vec![
                    Field {
                        name: "small".into(),
                        role: Type::Integer(Some(Range(0, 255))).untagged(),
                    },
                    Field {
                        name: "bigger".into(),
                        role: Type::Integer(Some(Range(0, 65535))).untagged(),
                    },
                    Field {
                        name: "negative".into(),
                        role: Type::Integer(Some(Range(-1, 255))).untagged(),
                    },
                    Field {
                        name: "unlimited".into(),
                        role: Type::Integer(None).optional().untagged(),
                    }
                ]))
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
                Type::Sequence(Sequence::from(vec![Field {
                    name: "decision".into(),
                    role: Type::Enumerated(Enumerated::from_names(
                        ["ABORT", "RETURN", "CONFIRM", "MAYDAY", "THE_CAKE_IS_A_LIE",].iter()
                    ))
                    .optional()
                    .untagged(),
                }]))
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
                Type::SequenceOf(Box::new(Type::Integer(Some(Range(0, 1))))).untagged(),
            ),
            model.definitions[0]
        );
        assert_eq!(
            Definition(
                "NestedOnes".into(),
                Type::SequenceOf(Box::new(Type::SequenceOf(Box::new(Type::Integer(Some(
                    Range(0, 1)
                ))))))
                .untagged(),
            ),
            model.definitions[1]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Type::Sequence(Sequence::from(vec![
                    Field {
                        name: "also-ones".into(),
                        role: Type::SequenceOf(Box::new(Type::Integer(Some(Range(0, 1)))))
                            .untagged(),
                    },
                    Field {
                        name: "nesteds".into(),
                        role: Type::SequenceOf(Box::new(Type::SequenceOf(Box::new(
                            Type::Integer(Some(Range(0, 1)))
                        ))))
                        .untagged(),
                    },
                    Field {
                        name: "optionals".into(),
                        role: Type::SequenceOf(Box::new(Type::SequenceOf(Box::new(
                            Type::Integer(None)
                        ))))
                        .optional()
                        .untagged(),
                    },
                ]))
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
                Type::SequenceOf(Box::new(Type::Integer(Some(Range(0, 1))))).untagged(),
            ),
            model.definitions[0]
        );
        assert_eq!(
            Definition(
                "That".into(),
                Type::SequenceOf(Box::new(Type::SequenceOf(Box::new(Type::Integer(Some(
                    Range(0, 1)
                ))))))
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
                Type::Sequence(Sequence::from(vec![Field {
                    name: "decision".into(),
                    role: Type::Choice(Choice::from(vec![
                        ChoiceVariant::name_type("this", Type::TypeReference("This".into())),
                        ChoiceVariant::name_type("that", Type::TypeReference("That".into())),
                        ChoiceVariant::name_type("neither", Type::TypeReference("Neither".into())),
                    ]))
                    .untagged(),
                }]))
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
                Type::Sequence(Sequence::from(vec![Field {
                    name: "complex".into(),
                    role: Type::Sequence(Sequence::from(vec![
                        Field {
                            name: "ones".into(),
                            role: Type::Integer(Some(Range(0, 1))).untagged(),
                        },
                        Field {
                            name: "list-ones".into(),
                            role: Type::SequenceOf(Box::new(Type::Integer(Some(Range(0, 1)))))
                                .untagged(),
                        },
                        Field {
                            name: "optional-ones".into(),
                            role: Type::SequenceOf(Box::new(Type::Integer(Some(Range(0, 1)))))
                                .optional()
                                .untagged(),
                        },
                    ]))
                    .optional()
                    .untagged(),
                }]))
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
                Type::Integer(Some(Range(0, 65_535))).untagged(),
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
                Type::UTF8String.untagged(),
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
    
            Application ::= [APPLICATION 7] SEQUENCE OF Utf8String
            
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
                    Type::Sequence(Sequence::from(vec![
                        Field {
                            name: "abc".to_string(),
                            role: Type::Integer(None).tagged(Tag::ContextSpecific(1)),
                        },
                        Field {
                            name: "def".to_string(),
                            role: Type::Integer(Some(Range(0, 255)))
                                .tagged(Tag::ContextSpecific(2)),
                        }
                    ]))
                    .tagged(Tag::Universal(2)),
                ),
                Definition(
                    "Application".to_string(),
                    Type::SequenceOf(Box::new(Type::UTF8String)).tagged(Tag::Application(7)),
                ),
                Definition(
                    "Private".to_string(),
                    Type::Enumerated(Enumerated::from_names(["abc", "def"].iter()))
                        .tagged(Tag::Private(11)),
                ),
                Definition(
                    "ContextSpecific".to_string(),
                    Type::Integer(None).tagged(Tag::ContextSpecific(8)),
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
                abc Utf8String,
                def Utf8String
            }
            
            WithoutExtensionPresent ::= CHOICE {
                abc Utf8String,
                def Utf8String,
                ...
            }
    
            WithExtensionPresent ::= CHOICE {
                abc Utf8String,
                def Utf8String,
                ...,
                ghi Utf8String
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
                            ChoiceVariant::name_type("abc", Type::UTF8String),
                            ChoiceVariant::name_type("def", Type::UTF8String),
                        ],
                        extension_after: None,
                    })
                    .untagged(),
                ),
                Definition::new(
                    "WithoutExtensionPresent",
                    Type::Choice(Choice {
                        variants: vec![
                            ChoiceVariant::name_type("abc", Type::UTF8String),
                            ChoiceVariant::name_type("def", Type::UTF8String),
                        ],
                        extension_after: Some(1),
                    })
                    .untagged(),
                ),
                Definition::new(
                    "WithExtensionPresent",
                    Type::Choice(Choice {
                        variants: vec![
                            ChoiceVariant::name_type("abc", Type::UTF8String),
                            ChoiceVariant::name_type("def", Type::UTF8String),
                            ChoiceVariant::name_type("ghi", Type::UTF8String),
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
                    abc Utf8String
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
}
