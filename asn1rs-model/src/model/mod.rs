pub mod protobuf;
pub mod rust;
pub mod sql;

pub use self::rust::Rust;
pub use self::rust::RustType;

pub use self::protobuf::Protobuf;
pub use self::protobuf::ProtobufType;

use crate::parser::{Location, Token};
use std::convert::TryFrom;
use std::fmt::Debug;
use std::iter::Peekable;
use std::vec::IntoIter;

macro_rules! loop_ctrl_separator {
    ($token:expr) => {
        match $token {
            t if t.eq_separator(',') => continue,
            t if t.eq_separator('}') => break,
            t => return Err(Error::unexpected_token(t)),
        }
    };
}

mod asn;
mod bit_string;
mod charset;
mod choice;
mod components;
mod definition;
mod enumerated;
mod err;
mod int;
mod itc;
pub mod lor;
mod oid;
mod parse;
mod range;
mod size;
mod tag;
mod tag_resolver;

use crate::model::itc::InnerTypeConstraints;
use crate::model::lor::{ResolveState, Resolved, Resolver, Unresolved};
pub use asn::Asn;
pub use asn::Type;
pub use bit_string::BitString;
pub use charset::Charset;
pub use choice::Choice;
pub use choice::ChoiceVariant;
pub use components::ComponentTypeList;
pub use definition::Definition;
pub use enumerated::Enumerated;
pub use enumerated::EnumeratedVariant;
pub use err::Error;
pub use err::ErrorKind;
pub use int::Integer;
pub use lor::Error as ResolveError;
pub use lor::LitOrRef;
pub use oid::{ObjectIdentifier, ObjectIdentifierComponent};
pub use parse::PeekableTokens;
pub use range::Range;
pub use size::Size;
pub use tag::Tag;
pub use tag::TagProperty;
pub use tag_resolver::TagResolver;

#[derive(Debug, Clone)]
pub struct Model<T: Target> {
    pub name: String,
    pub oid: Option<ObjectIdentifier>,
    pub imports: Vec<Import>,
    pub definitions: Vec<Definition<T::DefinitionType>>,
    pub value_references: Vec<ValueReference<T::ValueReferenceType>>,
}

pub trait Target {
    type DefinitionType;
    type ValueReferenceType;
}

impl<T: Target> Default for Model<T> {
    fn default() -> Self {
        Model {
            name: Default::default(),
            oid: None,
            imports: Default::default(),
            definitions: Default::default(),
            value_references: Vec::default(),
        }
    }
}

impl Model<Asn<Unresolved>> {
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
            } else if iter.peek_is_separator_eq(':') {
                model.definitions.push(Self::read_definition(
                    &mut iter,
                    token.into_text_or_else(Error::unexpected_token)?,
                )?);
            } else {
                model.value_references.push(Self::read_value_reference(
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
        if iter.next_is_separator_and_eq('{') {
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
                } else if iter.next_is_separator_and_eq('(') {
                    let number = match iter.next_text_or_err()?.parse::<u64>() {
                        Ok(number) => number,
                        Err(_) => return Err(Error::invalid_int_value(token)),
                    };
                    iter.next_separator_eq_or_err(')')?;
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
                let token = iter.next_or_err()?;
                if token.eq_separator(',') {
                    // ignore separator
                } else if token.eq_text_ignore_ascii_case("FROM") {
                    import.from = iter.next_text_or_err()?;
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
    ) -> Result<Definition<Asn<Unresolved>>, Error> {
        iter.next_separator_eq_or_err(':')?;
        iter.next_separator_eq_or_err(':')?;
        iter.next_separator_eq_or_err('=')?;

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

    fn read_value_reference<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
        name: String,
    ) -> Result<ValueReference<Asn<Unresolved>>, Error> {
        let r#type = Self::read_role(iter)?;
        Ok(ValueReference {
            name,
            value: {
                iter.next_separator_eq_or_err(':')?;
                iter.next_separator_eq_or_err(':')?;
                iter.next_separator_eq_or_err('=')?;
                Self::read_literal(iter, &r#type)?
            },
            role: Asn {
                tag: None,
                r#type,
                default: None,
            },
        })
    }

    fn read_literal<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
        r#type: &Type<Unresolved>,
    ) -> Result<LiteralValue, ErrorKind> {
        let location = iter.peek_or_err()?.location();
        let string = match r#type {
            Type::Boolean
                if iter.peek_is_text_eq_ignore_case("true")
                    || iter.peek_is_text_eq_ignore_case("false") =>
            {
                iter.next_text_or_err()?
            }
            Type::Integer(_)
                if iter.peek_is_text_and_satisfies(|slice| {
                    slice.chars().all(|c| c.is_ascii_digit())
                        || (slice.starts_with('-')
                            && slice.len() > 1
                            && slice.chars().skip(1).all(|c| c.is_ascii_digit()))
                }) =>
            {
                iter.next_text_or_err()?
            }
            Type::String(_, _) if iter.peek_is_separator_eq('"') => {
                Self::read_string_literal(iter, '"')?
            }
            Type::OctetString(_) | Type::BitString(_) if iter.peek_is_separator_eq('\'') => {
                Self::read_hex_or_bit_string_literal(iter)?
            }
            _ => {
                return Err(ErrorKind::UnsupportedLiteral(
                    iter.peek_or_err()?.clone(),
                    Box::new(r#type.clone()),
                ));
            }
        };
        LiteralValue::try_from_asn_str(&string)
            .ok_or(ErrorKind::InvalidLiteral(Token::Text(location, string)))
    }

    fn read_string_literal<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
        delimiter: char,
    ) -> Result<String, ErrorKind> {
        iter.next_separator_eq_or_err(delimiter)?;
        let token = iter.next_or_err()?;

        let first_text = token.text().unwrap_or_default();
        let mut string = String::from(delimiter);
        string.push_str(first_text);
        let mut prev_loc = Location::at(
            token.location().line(),
            token.location().column() + first_text.chars().count(),
        );

        loop {
            match iter.next_or_err()? {
                t if t.eq_separator(delimiter) => break,
                Token::Text(loc, str) => {
                    for _ in prev_loc.column()..loc.column() {
                        string.push(' ');
                    }
                    string.push_str(&str);
                    prev_loc = Location::at(loc.line(), loc.column() + str.chars().count())
                }
                Token::Separator(loc, char) => {
                    for _ in prev_loc.column()..loc.column() {
                        string.push(' ');
                    }
                    string.push(char);
                    prev_loc = Location::at(loc.line(), loc.column() + 1)
                }
            }
        }

        string.push(delimiter);

        Ok(string)
    }

    fn read_hex_or_bit_string_literal<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
    ) -> Result<String, ErrorKind> {
        let mut string = Self::read_string_literal(iter, '\'')?;
        match iter.next_text_eq_any_ignore_case_or_err(&["H", "B"])? {
            Token::Text(_, suffix) => string.push_str(&suffix),
            t => return Err(ErrorKind::UnexpectedToken(t)),
        };
        Ok(string)
    }

    fn next_with_opt_tag<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
    ) -> Result<(Token, Option<Tag>), Error> {
        let token = iter.next_or_err()?;
        if token.eq_separator('[') {
            let tag = Tag::try_from(&mut *iter)?;
            iter.next_separator_eq_or_err(']')?;
            let token = iter.next_or_err()?;
            Ok((token, Some(tag)))
        } else {
            Ok((token, None))
        }
    }

    fn read_role<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
    ) -> Result<Type<Unresolved>, Error> {
        let text = iter.next_text_or_err()?;
        Self::read_role_given_text(iter, text)
    }

    fn read_role_given_text<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
        text: String,
    ) -> Result<Type<Unresolved>, Error> {
        Ok(match text.to_ascii_lowercase().as_ref() {
            "integer" => Type::Integer(Integer::try_from(iter)?),
            "boolean" => Type::Boolean,
            "utf8string" => Type::String(Self::maybe_read_size(iter)?, Charset::Utf8),
            "ia5string" => Type::String(Self::maybe_read_size(iter)?, Charset::Ia5),
            "numericstring" => Type::String(Self::maybe_read_size(iter)?, Charset::Numeric),
            "printablestring" => Type::String(Self::maybe_read_size(iter)?, Charset::Printable),
            "visiblestring" => Type::String(Self::maybe_read_size(iter)?, Charset::Visible),
            "octet" => {
                iter.next_text_eq_ignore_case_or_err("STRING")?;
                Type::OctetString(Self::maybe_read_size(iter)?)
            }
            "bit" => {
                iter.next_text_eq_ignore_case_or_err("STRING")?;
                Type::BitString(BitString::try_from(iter)?)
            }
            "enumerated" => Type::Enumerated(Enumerated::try_from(iter)?),
            "choice" => Type::Choice(Choice::try_from(iter)?),
            "sequence" => Self::read_sequence_or_sequence_of(iter)?,
            "set" => Self::read_set_or_set_of(iter)?,
            _ => {
                let _ = Self::maybe_read_with_components_constraint(iter)?;
                Type::TypeReference(text, None)
            }
        })
    }

    fn maybe_read_with_components_constraint<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
    ) -> Result<Option<InnerTypeConstraints>, Error> {
        if iter.next_is_separator_and_eq('(') {
            let result = InnerTypeConstraints::try_from(&mut *iter)?;
            iter.next_separator_eq_or_err(')')?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    fn maybe_read_size<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
    ) -> Result<Size<<Unresolved as ResolveState>::SizeType>, Error> {
        if iter.next_is_separator_and_eq('(') {
            let result = Size::try_from(&mut *iter)?;
            iter.next_separator_eq_or_err(')')?;
            Ok(result)
        } else if iter.peek_is_text_eq_ignore_case("SIZE") {
            Size::try_from(iter)
        } else {
            Ok(Size::Any)
        }
    }

    fn read_sequence_or_sequence_of<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
    ) -> Result<Type<Unresolved>, Error> {
        let size = Self::maybe_read_size(iter)?;

        if iter.next_is_text_and_eq_ignore_case("OF") {
            Ok(Type::SequenceOf(Box::new(Self::read_role(iter)?), size))
        } else {
            Ok(Type::Sequence(ComponentTypeList::try_from(iter)?))
        }
    }

    fn read_set_or_set_of<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
    ) -> Result<Type<Unresolved>, Error> {
        let size = Self::maybe_read_size(iter)?;

        if iter.next_is_text_and_eq_ignore_case("OF") {
            Ok(Type::SetOf(Box::new(Self::read_role(iter)?), size))
        } else {
            Ok(Type::Set(ComponentTypeList::try_from(iter)?))
        }
    }

    fn read_field<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
    ) -> Result<(Field<Asn<Unresolved>>, bool), Error> {
        let name = iter.next_text_or_err()?;
        let (token, tag) = Self::next_with_opt_tag(iter)?;
        let mut field = Field {
            name,
            role: Self::read_role_given_text(iter, token.into_text_or_else(Error::no_text)?)?
                .opt_tagged(tag),
        };

        let token = {
            let token = iter.next_or_err()?;
            if token.eq_text_ignore_ascii_case("OPTIONAL") {
                field.role.make_optional();
                iter.next_or_err()?
            } else if token.eq_text_ignore_ascii_case("DEFAULT") {
                if cfg!(feature = "debug-proc-macro") {
                    println!("TOKEN:::: {:?}", token);
                }
                field
                    .role
                    .set_default(match Self::read_literal(iter, &field.role.r#type) {
                        Ok(value) => LitOrRef::Lit(value),
                        Err(ErrorKind::UnsupportedLiteral(token, ..)) if token.is_text() => {
                            LitOrRef::Ref(iter.next_text_or_err()?)
                        }
                        Err(e) => return Err(e.into()),
                    });
                if cfg!(feature = "debug-proc-macro") {
                    println!("     :::: {:?}", field);
                }
                iter.next_or_err()?
            } else {
                token
            }
        };

        let (continues, ends) = token
            .separator()
            .map_or((false, false), |s| (s == ',', s == '}'));

        if continues || ends {
            Ok((field, continues))
        } else {
            Err(Error::unexpected_token(token))
        }
    }
}

impl Model<Asn<Resolved>> {
    pub fn to_rust(&self) -> Model<rust::Rust> {
        let scope: &[&Self] = &[];
        Model::convert_asn_to_rust(self, scope)
    }

    pub fn to_rust_with_scope(&self, scope: &[&Self]) -> Model<rust::Rust> {
        Model::convert_asn_to_rust(self, scope)
    }
}

impl<RS: ResolveState> Model<Asn<RS>> {
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

    fn maybe_read_constants<R, F: Fn(Token) -> Result<R, Error>, T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
        parser: F,
    ) -> Result<Vec<(String, R)>, Error> {
        let mut constants = Vec::default();
        if iter.next_is_separator_and_eq('{') {
            loop {
                constants.push(Self::read_constant(iter, |token| parser(token))?);
                loop_ctrl_separator!(iter.next_or_err()?);
            }
        }
        Ok(constants)
    }

    fn read_constant<R, F: Fn(Token) -> Result<R, Error>, T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
        parser: F,
    ) -> Result<(String, R), Error> {
        let name = iter.next_text_or_err()?;
        iter.next_separator_eq_or_err('(')?;
        let value = iter.next_or_err()?;
        iter.next_separator_eq_or_err(')')?;
        Ok((name, parser(value)?))
    }

    fn constant_i64_parser(token: Token) -> Result<i64, Error> {
        let parsed = token.text().and_then(|s| s.parse().ok());
        parsed.ok_or_else(|| Error::invalid_value_for_constant(token))
    }

    fn constant_u64_parser(token: Token) -> Result<u64, Error> {
        let parsed = token.text().and_then(|s| s.parse().ok());
        parsed.ok_or_else(|| Error::invalid_value_for_constant(token))
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ValueReference<T> {
    pub name: String,
    pub role: T,
    pub value: LiteralValue,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum LiteralValue {
    Boolean(bool),
    String(String),
    Integer(i64),
    OctetString(Vec<u8>),
    EnumeratedVariant(String, String),
}

impl LiteralValue {
    pub fn to_integer(&self) -> Option<i64> {
        if let LiteralValue::Integer(int) = self {
            Some(*int)
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Clone, PartialOrd, PartialEq)]
pub struct Import {
    pub what: Vec<String>,
    pub from: String,
    pub from_oid: Option<ObjectIdentifier>,
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

impl Field<Asn<Unresolved>> {
    pub fn try_resolve<
        R: Resolver<<Resolved as ResolveState>::SizeType>
            + Resolver<<Resolved as ResolveState>::RangeType>
            + Resolver<<Resolved as ResolveState>::ConstType>
            + Resolver<Type<Unresolved>>,
    >(
        &self,
        resolver: &R,
    ) -> Result<Field<Asn<Resolved>>, ResolveError> {
        Ok(Field {
            name: self.name.clone(),
            role: self.role.try_resolve(resolver)?,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::parser::{Location, Tokenizer};

    use super::*;

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
        let model = Model::try_from(Tokenizer::default().parse(SIMPLE_INTEGER_STRUCT_ASN))
            .unwrap()
            .try_resolve()
            .unwrap();

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
        let model = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_ENUM))
            .unwrap()
            .try_resolve()
            .unwrap();

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
        let model = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_SEQUENCE_OF))
            .unwrap()
            .try_resolve()
            .unwrap();

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
        let model = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_CHOICE))
            .unwrap()
            .try_resolve()
            .unwrap();

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
        let model = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_SEQUENCE))
            .unwrap()
            .try_resolve()
            .unwrap();

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
        .expect("Failed to parse")
        .try_resolve()
        .expect("Failed to resolve");

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
        .expect("Failed to parse")
        .try_resolve()
        .expect("Failed to resolve");

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
        .expect("Failed to parse")
        .try_resolve()
        .expect("Failed to resolve");

        assert_eq!("SimpleSchema", &model.name);
        assert_eq!(
            &[
                Definition(
                    "Basic".to_string(),
                    Type::Enumerated(Enumerated::from_names(["abc", "def"].iter())).untagged(),
                ),
                Definition(
                    "WithExplicitNumber".to_string(),
                    Type::Enumerated(Enumerated::from(vec![
                        EnumeratedVariant::from_name_number("abc", 1),
                        EnumeratedVariant::from_name_number("def", 9)
                    ]))
                    .untagged(),
                ),
                Definition(
                    "WithExplicitNumberAndDefaultMark".to_string(),
                    Type::Enumerated(
                        Enumerated::from(vec![
                            EnumeratedVariant::from_name_number("abc", 4),
                            EnumeratedVariant::from_name_number("def", 7),
                        ],)
                        .with_extension_after(1)
                    )
                    .untagged(),
                ),
                Definition(
                    "WithExplicitNumberAndDefaultMarkV2".to_string(),
                    Type::Enumerated(
                        Enumerated::from(vec![
                            EnumeratedVariant::from_name_number("abc", 8),
                            EnumeratedVariant::from_name_number("def", 1),
                            EnumeratedVariant::from_name_number("v2", 11)
                        ],)
                        .with_extension_after(1)
                    )
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
        .expect("Failed to parse")
        .try_resolve()
        .expect("Failed to resolve");

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
        .expect("Failed to parse")
        .try_resolve()
        .expect("Failed to resolve");

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
        .expect("Failed to parse")
        .try_resolve()
        .expect("Failed to resolve");

        assert_eq!("SimpleSchema", model.name.as_str());
        assert_eq!(
            &[
                Definition::new(
                    "WithoutMarker",
                    Type::Choice(Choice::from(vec![
                        ChoiceVariant::name_type("abc", Type::unconstrained_utf8string()),
                        ChoiceVariant::name_type("def", Type::unconstrained_utf8string()),
                    ]))
                    .untagged(),
                ),
                Definition::new(
                    "WithoutExtensionPresent",
                    Type::Choice(
                        Choice::from(vec![
                            ChoiceVariant::name_type("abc", Type::unconstrained_utf8string()),
                            ChoiceVariant::name_type("def", Type::unconstrained_utf8string()),
                        ])
                        .with_extension_after(1),
                    )
                    .untagged(),
                ),
                Definition::new(
                    "WithExtensionPresent",
                    Type::Choice(
                        Choice::from(vec![
                            ChoiceVariant::name_type("abc", Type::unconstrained_utf8string()),
                            ChoiceVariant::name_type("def", Type::unconstrained_utf8string()),
                            ChoiceVariant::name_type("ghi", Type::unconstrained_utf8string()),
                        ])
                        .with_extension_after(1),
                    )
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
        .expect("Failed to load model")
        .try_resolve()
        .expect("Failed to resolve");
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
        .expect("Failed to load model")
        .try_resolve()
        .expect("Failed to resolve");
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
        .expect("Failed to parse module")
        .try_resolve()
        .expect("Failed to resolve");
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
        .expect("Failed to load model")
        .try_resolve()
        .expect("Failed to resolve");
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

    #[test]
    pub fn test_value_reference_boolean() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN
                
                somethingYes BOOLEAN ::= TRUE
                somethingNo BOOLEAN ::= FALSE
                
                END",
        ))
        .expect("Failed to load model");
        assert_eq!(
            &[
                ValueReference {
                    name: "somethingYes".to_string(),
                    role: Type::Boolean.untagged(),
                    value: LiteralValue::Boolean(true)
                },
                ValueReference {
                    name: "somethingNo".to_string(),
                    role: Type::Boolean.untagged(),
                    value: LiteralValue::Boolean(false)
                },
            ],
            &model.value_references[..]
        )
    }

    #[test]
    pub fn test_value_reference_integer() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN
                
                maxSomethingSomething INTEGER ::= 1337
                
                END",
        ))
        .expect("Failed to load model");
        assert_eq!(
            ValueReference {
                name: "maxSomethingSomething".to_string(),
                role: Type::Integer(Integer {
                    range: Default::default(),
                    constants: Vec::default()
                })
                .untagged(),
                value: LiteralValue::Integer(1337)
            },
            model.value_references[0]
        )
    }

    #[test]
    pub fn test_value_reference_bit_string() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN
                
                magicFlags BIT STRING ::= 'a711'H
                
                magicFlags2 BIT STRING ::= '1001'B
                
                END",
        ))
        .expect("Failed to load model");
        assert_eq!(
            ValueReference {
                name: "magicFlags".to_string(),
                role: Type::BitString(BitString {
                    size: Size::Any,
                    constants: Vec::default()
                })
                .untagged(),
                value: LiteralValue::OctetString(vec![0xa7, 0x11])
            },
            model.value_references[0]
        );
        assert_eq!(
            ValueReference {
                name: "magicFlags2".to_string(),
                role: Type::BitString(BitString {
                    size: Size::Any,
                    constants: Vec::default()
                })
                .untagged(),
                value: LiteralValue::OctetString(vec![0x09])
            },
            model.value_references[1]
        );
    }

    #[test]
    pub fn test_value_reference_octet_string() {
        let model = Model::try_from(Tokenizer::default().parse(
            r"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN

                answers OCTET STRING ::= '42'h

                END",
        ))
        .expect("Failed to load model");
        assert_eq!(
            ValueReference {
                name: "answers".to_string(),
                role: Type::OctetString(Size::Any).untagged(),
                value: LiteralValue::OctetString(vec![0x42])
            },
            model.value_references[0]
        )
    }

    #[test]
    pub fn test_value_reference_string() {
        let model = Model::try_from(Tokenizer::default().parse(
            r#"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN

                utf8 UTF8String ::= "häw äre yöu .. .. doing"
                ia5 IA5String ::= "how are you"

                END"#,
        ))
        .expect("Failed to load model");
        assert_eq!(
            &[
                ValueReference {
                    name: "utf8".to_string(),
                    role: Type::String(Size::Any, Charset::Utf8).untagged(),
                    value: LiteralValue::String("häw äre yöu .. .. doing".to_string())
                },
                ValueReference {
                    name: "ia5".to_string(),
                    role: Type::String(Size::Any, Charset::Ia5).untagged(),
                    value: LiteralValue::String("how are you".to_string())
                }
            ],
            &model.value_references[..]
        );
    }

    #[test]
    pub fn test_value_reference_in_size() {
        let model = Model::try_from(Tokenizer::default().parse(
            r#"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN

                se_min INTEGER ::= 42
                se_max INTEGER ::= 1337
                
                seq-fix         ::= SEQUENCE (SIZE(se_min)) OF INTEGER
                seq-min-max     ::= SEQUENCE (SIZE(se_min..se_max)) OF INTEGER
                seq-min-max-ext ::= SEQUENCE (SIZE(se_min..se_max,...)) OF INTEGER
                
                mixed-min-max     ::= SEQUENCE (SIZE(se_min..4711)) OF INTEGER
                mixed-min-max-ext ::= SEQUENCE (SIZE(420..se_max,...)) OF INTEGER

                END"#,
        ))
        .expect("Failed to load model")
        .try_resolve()
        .expect("Failed to resolve");
        assert_eq!(
            &[
                Definition(
                    "seq-fix".to_string(),
                    Type::<Resolved>::SequenceOf(
                        Box::new(Type::Integer(Integer::default())),
                        Size::Fix(42_usize, false)
                    )
                    .untagged()
                ),
                Definition(
                    "seq-min-max".to_string(),
                    Type::<Resolved>::SequenceOf(
                        Box::new(Type::Integer(Integer::default())),
                        Size::Range(42_usize, 1337, false)
                    )
                    .untagged()
                ),
                Definition(
                    "seq-min-max-ext".to_string(),
                    Type::<Resolved>::SequenceOf(
                        Box::new(Type::Integer(Integer::default())),
                        Size::Range(42_usize, 1337, true)
                    )
                    .untagged()
                ),
                Definition(
                    "mixed-min-max".to_string(),
                    Type::<Resolved>::SequenceOf(
                        Box::new(Type::Integer(Integer::default())),
                        Size::Range(42_usize, 4711, false)
                    )
                    .untagged()
                ),
                Definition(
                    "mixed-min-max-ext".to_string(),
                    Type::<Resolved>::SequenceOf(
                        Box::new(Type::Integer(Integer::default())),
                        Size::Range(420_usize, 1337, true)
                    )
                    .untagged()
                )
            ],
            &model.definitions[..]
        );
    }

    #[test]
    pub fn test_value_reference_in_range() {
        let model = Model::try_from(Tokenizer::default().parse(
            r#"SomeName DEFINITIONS AUTOMATIC TAGS ::= BEGIN

                se_min INTEGER ::= 42
                se_max INTEGER ::= 1337
                
                seq-min-max     ::= INTEGER(se_min..se_max)
                seq-min-max-ext ::= INTEGER(se_min..se_max,...)
                
                mixed-min-max     ::= INTEGER(se_min..4711)
                mixed-min-max-ext ::= INTEGER(-42069..se_max,...)

                END"#,
        ))
        .expect("Failed to load model")
        .try_resolve()
        .expect("Failed to resolve");
        assert_eq!(
            &[
                Definition(
                    "seq-min-max".to_string(),
                    Type::<Resolved>::Integer(Integer::with_range(Range::inclusive(
                        Some(42),
                        Some(1337)
                    )))
                    .untagged()
                ),
                Definition(
                    "seq-min-max-ext".to_string(),
                    Type::<Resolved>::Integer(Integer::with_range(
                        Range::inclusive(Some(42), Some(1337)).with_extensible(true)
                    ))
                    .untagged()
                ),
                Definition(
                    "mixed-min-max".to_string(),
                    Type::<Resolved>::Integer(Integer::with_range(Range::inclusive(
                        Some(42),
                        Some(4711)
                    )))
                    .untagged()
                ),
                Definition(
                    "mixed-min-max-ext".to_string(),
                    Type::<Resolved>::Integer(Integer::with_range(
                        Range::inclusive(Some(-42069), Some(1337)).with_extensible(true)
                    ))
                    .untagged()
                )
            ],
            &model.definitions[..]
        );
    }
}
