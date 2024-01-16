use crate::asn::oid::{ObjectIdentifier, ObjectIdentifierComponent};
use crate::asn::resolve_scope::ResolveScope;
use crate::asn::{Asn, ComponentTypeList, InnerTypeConstraints, Size, Tag, Type};
use crate::asn::{BitString, Charset, Choice, Enumerated, Integer};
use crate::model::err::{Error, ErrorKind};
use crate::model::lit_or_ref::{LitOrRef, ResolveState, Resolved, Resolver, Unresolved};
use crate::model::parse::PeekableTokens;
use crate::model::{rust, Field, Import, LiteralValue, Model, ValueReference};
use crate::parser::{Location, Token};
use std::convert::TryFrom;
use std::iter::Peekable;
use std::vec::IntoIter;

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
    ) -> Result<crate::model::Definition<Asn<Unresolved>>, Error> {
        iter.next_separator_eq_or_err(':')?;
        iter.next_separator_eq_or_err(':')?;
        iter.next_separator_eq_or_err('=')?;

        let (token, tag) = Self::next_with_opt_tag(iter)?;

        if token.eq_text_ignore_ascii_case("SEQUENCE") {
            Ok(crate::model::Definition(
                name,
                Self::read_sequence_or_sequence_of(iter)?.opt_tagged(tag),
            ))
        } else if token.eq_text_ignore_ascii_case("SET") {
            Ok(crate::model::Definition(
                name,
                Self::read_set_or_set_of(iter)?.opt_tagged(tag),
            ))
        } else if token.eq_text_ignore_ascii_case("ENUMERATED") {
            Ok(crate::model::Definition(
                name,
                Type::Enumerated(Enumerated::try_from(iter)?).opt_tagged(tag),
            ))
        } else if token.eq_text_ignore_ascii_case("CHOICE") {
            Ok(crate::model::Definition(
                name,
                Type::Choice(Choice::try_from(iter)?).opt_tagged(tag),
            ))
        } else if let Some(text) = token.text() {
            Ok(crate::model::Definition(
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
                Self::read_literal(iter)?
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
    ) -> Result<LiteralValue, ErrorKind> {
        let location = iter.peek_or_err()?.location();
        let string = {
            // boolean or integer
            #[allow(clippy::blocks_in_if_conditions)]
            if iter.peek_is_text_eq_ignore_case("true")
                || iter.peek_is_text_eq_ignore_case("false")
                || iter.peek_is_text_and_satisfies(|slice| {
                    slice.chars().all(|c| c.is_ascii_digit())
                        || (slice.starts_with('-')
                            && slice.len() > 1
                            && slice.chars().skip(1).all(|c| c.is_ascii_digit()))
                })
            {
                iter.next_text_or_err()?
            } else if iter.peek_is_separator_eq('"') {
                Self::read_string_literal(iter, '"')?
            } else if iter.peek_is_separator_eq('\'') {
                Self::read_hex_or_bit_string_literal(iter)?
            } else {
                return Err(ErrorKind::UnsupportedLiteral(iter.peek_or_err()?.clone()));
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

    pub(crate) fn next_with_opt_tag<T: Iterator<Item = Token>>(
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

    pub(crate) fn read_role_given_text<T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
        text: String,
    ) -> Result<Type<Unresolved>, Error> {
        Ok(match text.to_ascii_lowercase().as_ref() {
            "integer" => Type::Integer(Integer::try_from(iter)?),
            "boolean" => Type::Boolean,
            "null" => Type::Null,
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
                // TODO use InnerTypeConstraints to flatten TypeReference to an actual type and
                //      prevent tuple-type nesting in the generated rust and other code by copying
                //      over the fields and adding these additional constraints
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

    pub(crate) fn maybe_read_size<T: Iterator<Item = Token>>(
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

    pub(crate) fn read_field<T: Iterator<Item = Token>>(
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
                field.role.set_default(match Self::read_literal(iter) {
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
        Model::to_rust_with_scope(self, scope)
    }

    pub fn to_rust_keep_names(&self) -> Model<rust::Rust> {
        let scope: &[&Self] = &[];
        Model::to_rust_keep_names_with_scope(self, scope)
    }

    pub fn to_rust_with_scope(&self, scope: &[&Self]) -> Model<rust::Rust> {
        Model::convert_asn_to_rust(self, scope, true)
    }

    pub fn to_rust_keep_names_with_scope(&self, scope: &[&Self]) -> Model<rust::Rust> {
        Model::convert_asn_to_rust(self, scope, false)
    }
}

impl<RS: ResolveState> Model<Asn<RS>> {
    pub fn make_names_nice(&mut self) {
        Self::make_name_nice(&mut self.name);
        for import in &mut self.imports {
            Self::make_name_nice(&mut import.from);
        }
    }

    pub fn make_name_nice(name: &mut String) {
        const TO_REMOVE_AT_END: &[&str] = &["_Module", "Module"];
        for to_remove in TO_REMOVE_AT_END.iter() {
            if name.ends_with(to_remove) {
                let new_len = name.len() - to_remove.len();
                name.truncate(new_len);
            }
        }
    }

    pub(crate) fn maybe_read_constants<
        R,
        F: Fn(Token) -> Result<R, Error>,
        T: Iterator<Item = Token>,
    >(
        iter: &mut Peekable<T>,
        parser: F,
    ) -> Result<Vec<(String, R)>, Error> {
        let mut constants = Vec::default();
        if iter.next_is_separator_and_eq('{') {
            loop {
                constants.push(Self::read_constant(iter, &parser)?);
                loop_ctrl_separator!(iter.next_or_err()?);
            }
        }
        Ok(constants)
    }

    pub(crate) fn read_constant<R, F: Fn(Token) -> Result<R, Error>, T: Iterator<Item = Token>>(
        iter: &mut Peekable<T>,
        parser: F,
    ) -> Result<(String, R), Error> {
        let name = iter.next_text_or_err()?;
        iter.next_separator_eq_or_err('(')?;
        let value = iter.next_or_err()?;
        iter.next_separator_eq_or_err(')')?;
        Ok((name, parser(value)?))
    }

    pub(crate) fn constant_i64_parser(token: Token) -> Result<i64, Error> {
        let parsed = token.text().and_then(|s| s.parse().ok());
        parsed.ok_or_else(|| Error::invalid_value_for_constant(token))
    }

    pub(crate) fn constant_u64_parser(token: Token) -> Result<u64, Error> {
        let parsed = token.text().and_then(|s| s.parse().ok());
        parsed.ok_or_else(|| Error::invalid_value_for_constant(token))
    }
}

impl Model<Asn<Unresolved>> {
    #[inline]
    pub fn try_resolve(&self) -> Result<Model<Asn<Resolved>>, crate::model::lit_or_ref::Error> {
        ResolveScope::from(self).try_resolve()
    }
}

impl Field<Asn<Unresolved>> {
    #[inline]
    pub fn try_resolve<
        R: Resolver<<Resolved as ResolveState>::SizeType>
            + Resolver<<Resolved as ResolveState>::RangeType>
            + Resolver<<Resolved as ResolveState>::ConstType>
            + Resolver<Type<Unresolved>>,
    >(
        &self,
        resolver: &R,
    ) -> Result<Field<Asn<Resolved>>, crate::model::lit_or_ref::Error> {
        Ok(Field {
            name: self.name.clone(),
            role: self.role.try_resolve(resolver)?,
        })
    }
}
