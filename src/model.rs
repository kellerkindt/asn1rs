use backtrace::Backtrace;

use parser::Token;

use std::vec::IntoIter;

#[derive(Debug)]
pub enum Error {
    ExpectedTextGot(Backtrace, String, String),
    ExpectedSeparatorGot(Backtrace, char, char),
    UnexpectedToken(Backtrace, Token),
    MissingModuleName,
    UnexpectedEndOfStream,
    InvalidRangeValue,
}

#[derive(Debug, Clone)]
pub struct Model<T> {
    pub name: String,
    pub imports: Vec<Import>,
    pub definitions: Vec<Definition<T>>,
}

impl<T> Default for Model<T> {
    fn default() -> Self {
        Model {
            name: Default::default(),
            imports: Default::default(),
            definitions: Default::default(),
        }
    }
}

impl Model<Asn> {
    pub fn new<I: Into<String>>(name: I) -> Self {
        let mut model = Model {
            name: name.into(),
            imports: Default::default(),
            definitions: Default::default(),
        };
        model.make_names_nice();
        model
    }

    pub fn try_from(value: Vec<Token>) -> Result<Self, Error> {
        let mut model = Model::default();
        let mut iter = value.into_iter();

        model.name = Self::read_name(&mut iter)?;
        Self::skip_after(&mut iter, &Token::Text("BEGIN".into()))?;

        while let Some(token) = iter.next() {
            match token {
                t @ Token::Separator(_) => return Err(Error::UnexpectedToken(Backtrace::new(), t)),
                Token::Text(text) => {
                    let lower = text.to_lowercase();

                    if lower.eq(&"end") {
                        model.make_names_nice();
                        return Ok(model);
                    } else if lower.eq(&"imports") {
                        Self::read_imports(&mut iter)?
                            .into_iter()
                            .for_each(|i| model.imports.push(i));
                    } else {
                        model
                            .definitions
                            .push(Self::read_definition(&mut iter, text)?);
                    }
                }
            }
        }
        Err(Error::UnexpectedEndOfStream)
    }

    fn read_name(iter: &mut IntoIter<Token>) -> Result<String, Error> {
        iter.next()
            .and_then(|token| {
                if let Token::Text(text) = token {
                    Some(text)
                } else {
                    None
                }
            }).ok_or(Error::MissingModuleName)
    }

    fn skip_after(iter: &mut IntoIter<Token>, token: &Token) -> Result<(), Error> {
        while let Some(t) = iter.next() {
            if t.eq(&token) {
                return Ok(());
            }
        }
        Err(Error::UnexpectedEndOfStream)
    }

    fn read_imports(iter: &mut IntoIter<Token>) -> Result<Vec<Import>, Error> {
        let mut imports = Vec::new();
        let mut import = Import::default();
        while let Some(token) = iter.next() {
            match token {
                Token::Separator(s) if s == ';' => {
                    return Ok(imports);
                }
                Token::Text(text) => {
                    import.what.push(text);
                    match iter.next().ok_or(Error::UnexpectedEndOfStream)? {
                        Token::Separator(s) if s == ',' => {}
                        Token::Text(s) => {
                            let lower = s.to_lowercase();
                            if s.eq(&",") {

                            } else if lower.eq(&"from") {
                                let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
                                if let Token::Text(from) = token {
                                    import.from = from;
                                    imports.push(import);
                                    import = Import::default();
                                } else {
                                    return Err(Error::UnexpectedToken(Backtrace::new(), token));
                                }
                            }
                        }
                        t => return Err(Error::UnexpectedToken(Backtrace::new(), t)),
                    }
                }
                _ => return Err(Error::UnexpectedToken(Backtrace::new(), token)),
            }
        }
        Err(Error::UnexpectedEndOfStream)
    }
    fn read_definition(iter: &mut IntoIter<Token>, name: String) -> Result<Definition<Asn>, Error> {
        Self::next_separator_ignore_case(iter, ':')?;
        Self::next_separator_ignore_case(iter, ':')?;
        Self::next_separator_ignore_case(iter, '=')?;

        let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;

        if token.text().map(|s| s.eq(&"SEQUENCE")).unwrap_or(false) {
            Ok(Definition(name, Self::read_sequence_or_sequence_of(iter)?))
        } else if token
            .text()
            .map(|s| s.eq_ignore_ascii_case(&"ENUMERATED"))
            .unwrap_or(false)
        {
            Ok(Definition(
                name,
                Asn::Enumerated(Self::read_enumerated(iter)?),
            ))
        } else if token
            .text()
            .map(|s| s.eq_ignore_ascii_case(&"CHOICE"))
            .unwrap_or(false)
        {
            Ok(Definition(name, Asn::Choice(Self::read_choice(iter)?)))
        } else {
            Err(Error::UnexpectedToken(Backtrace::new(), token))
        }
    }

    fn read_role(iter: &mut IntoIter<Token>) -> Result<Asn, Error> {
        let text = Self::next_text(iter)?;
        if text.eq_ignore_ascii_case(&"INTEGER") {
            Self::next_separator_ignore_case(iter, '(')?;
            let start = Self::next_text(iter)?;
            Self::next_separator_ignore_case(iter, '.')?;
            Self::next_separator_ignore_case(iter, '.')?;
            let end = Self::next_text(iter)?;
            Self::next_separator_ignore_case(iter, ')')?;
            if start.eq("0") && end.eq("MAX") {
                Ok(Asn::Integer(None))
            } else if end.eq("MAX") {
                Err(Error::UnexpectedToken(
                    Backtrace::new(),
                    Token::Text("MAX".into()),
                ))
            } else {
                Ok(Asn::Integer(Some(Range(
                    start.parse::<i64>().map_err(|_| Error::InvalidRangeValue)?,
                    end.parse::<i64>().map_err(|_| Error::InvalidRangeValue)?,
                ))))
            }
        } else if text.eq_ignore_ascii_case(&"BOOLEAN") {
            Ok(Asn::Boolean)
        } else if text.eq_ignore_ascii_case(&"UTF8String") {
            Ok(Asn::UTF8String)
        } else if text.eq_ignore_ascii_case(&"OCTET") {
            let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
            if token.text().map(|t| t.eq("STRING")).unwrap_or(false) {
                Ok(Asn::OctetString)
            } else {
                Err(Error::UnexpectedToken(Backtrace::new(), token))
            }
        } else if text.eq_ignore_ascii_case(&"CHOICE") {
            Ok(Asn::Choice(Self::read_choice(iter)?))
        } else if text.eq_ignore_ascii_case(&"ENUMERATED") {
            Ok(Asn::Enumerated(Self::read_enumerated(iter)?))
        } else if text.eq_ignore_ascii_case(&"SEQUENCE") {
            Ok(Self::read_sequence_or_sequence_of(iter)?)
        } else {
            Ok(Asn::TypeReference(text))
        }
    }

    fn read_sequence_or_sequence_of(iter: &mut IntoIter<Token>) -> Result<Asn, Error> {
        let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
        match token {
            Token::Text(of) => {
                if of.eq_ignore_ascii_case(&"OF") {
                    Ok(Asn::SequenceOf(Box::new(Self::read_role(iter)?)))
                } else {
                    Err(Error::UnexpectedToken(Backtrace::new(), Token::Text(of)))
                }
            }
            Token::Separator(separator) => {
                if separator == '{' {
                    let mut fields = Vec::new();

                    loop {
                        let (field, continues) = Self::read_field(iter)?;
                        fields.push(field);
                        if !continues {
                            break;
                        }
                    }

                    Ok(Asn::Sequence(fields))
                } else {
                    Err(Error::UnexpectedToken(
                        Backtrace::new(),
                        Token::Separator(separator),
                    ))
                }
            }
        }
    }

    fn read_enumerated(iter: &mut IntoIter<Token>) -> Result<Vec<String>, Error> {
        Self::next_separator_ignore_case(iter, '{')?;
        let mut enumeration = Vec::new();

        loop {
            enumeration.push(Self::next_text(iter)?);
            let separator = Self::next_seperator(iter)?;
            if separator == '}' {
                break;
            }
        }

        Ok(enumeration)
    }

    fn read_choice(iter: &mut IntoIter<Token>) -> Result<Vec<ChoiceEntry>, Error> {
        Self::next_separator_ignore_case(iter, '{')?;
        let mut fields = Vec::new();

        loop {
            let (field, continues) = Self::read_field(iter)?;
            fields.push(ChoiceEntry(field.name, field.role));
            if !continues {
                break;
            }
        }

        Ok(fields)
    }

    fn read_field(iter: &mut IntoIter<Token>) -> Result<(Field<Asn>, bool), Error> {
        let mut field = Field {
            name: Self::next_text(iter)?,
            role: Self::read_role(iter)?,
            optional: false,
        };
        let mut token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
        if let Some(_optional_flag) = token.text().map(|s| s.eq_ignore_ascii_case(&"OPTIONAL")) {
            field.optional = true;
            token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
        }

        let (continues, ends) = token
            .separator()
            .map(|s| (s == ',', s == '}'))
            .unwrap_or((false, false));

        if continues || ends {
            Ok((field, continues))
        } else {
            Err(Error::UnexpectedToken(Backtrace::new(), token))
        }
    }

    fn next_text(iter: &mut IntoIter<Token>) -> Result<String, Error> {
        match iter.next().ok_or(Error::UnexpectedEndOfStream)? {
            Token::Text(text) => Ok(text),
            t => Err(Error::UnexpectedToken(Backtrace::new(), t)),
        }
    }

    #[allow(unused)]
    fn next_text_ignore_case(iter: &mut IntoIter<Token>, text: &str) -> Result<(), Error> {
        let token = Self::next_text(iter)?;
        if text.eq_ignore_ascii_case(&token) {
            Ok(())
        } else {
            Err(Error::ExpectedTextGot(Backtrace::new(), text.into(), token))
        }
    }

    fn next_seperator(iter: &mut IntoIter<Token>) -> Result<char, Error> {
        match iter.next().ok_or(Error::UnexpectedEndOfStream)? {
            Token::Separator(separator) => Ok(separator),
            t => Err(Error::UnexpectedToken(Backtrace::new(), t)),
        }
    }

    fn next_separator_ignore_case(iter: &mut IntoIter<Token>, text: char) -> Result<(), Error> {
        let token = Self::next_seperator(iter)?;
        if token.eq_ignore_ascii_case(&text) {
            Ok(())
        } else {
            Err(Error::ExpectedSeparatorGot(
                Backtrace::new(),
                text.into(),
                token,
            ))
        }
    }

    pub fn make_names_nice(&mut self) {
        Self::make_name_nice(&mut self.name);
        for import in self.imports.iter_mut() {
            Self::make_name_nice(&mut import.from);
        }
    }

    fn make_name_nice(name: &mut String) {
        const TO_REMOVE_AT_END: &[&'static str] = &["_Module", "Module"];
        for to_remove in TO_REMOVE_AT_END.iter() {
            if name.ends_with(to_remove) {
                let new_len = name.len() - to_remove.len();
                name.truncate(new_len);
            }
        }
    }

    pub fn to_rust(&self) -> Model<Rust> {
        let mut model = Model {
            name: rust_module_name(&self.name),
            imports: self.imports.clone(),
            definitions: Vec::with_capacity(self.definitions.len()),
        };
        for Definition(name, asn) in self.definitions.iter() {
            let rust_name = rust_struct_or_enum_name(name);
            Self::definition_to_rust(&rust_name, asn, &mut model.definitions);
        }
        model
    }

    pub fn definition_to_rust(name: &str, asn: &Asn, defs: &mut Vec<Definition<Rust>>) -> Rust {
        match asn {
            Asn::Boolean => Rust::Bool,
            Asn::Integer(Some(Range(min, max))) => {
                let min = *min;
                let max = *max;
                if min >= 0 {
                    match max as u64 {
                        0...U8_MAX => Rust::U8(Range(min as u8, max as u8)),
                        0...U16_MAX => Rust::U16(Range(min as u16, max as u16)),
                        0...U32_MAX => Rust::U32(Range(min as u32, max as u32)),
                        0...U64_MAX => Rust::U64(Some(Range(min as u64, max as u64))),
                        _ => panic!("This should never happen, since max (as u64 frm i64) cannot be greater than U64_MAX")
                    }
                } else {
                    let max_amplitude = (min - 1).abs().max(max);
                    match max_amplitude {
                        0...I8_MAX => Rust::I8(Range(min as i8, max as i8)),
                        0...I16_MAX => Rust::I16(Range(min as i16, max as i16)),
                        0...I32_MAX => Rust::I32(Range(min as i32, max as i32)),
                        0...I64_MAX => Rust::I64(Range(min as i64, max as i64)),
                        _ => panic!("This should never happen, since max (being i64) cannot be greater than I64_MAX")
                    }
                }
            }
            Asn::Integer(None) => Rust::U64(None),
            Asn::UTF8String => Rust::String,
            Asn::OctetString => Rust::VecU8,
            Asn::TypeReference(name) => Rust::Complex(name.clone()),

            Asn::Sequence(fields) => {
                let mut rust_fields = Vec::with_capacity(fields.len());
                let name = rust_struct_or_enum_name(name);

                for field in fields.iter() {
                    let rust_name = format!("{}{}", name, rust_struct_or_enum_name(&field.name));
                    let rust_role =
                        Self::unfold_asn_sequence_of_to_rust_vec(&rust_name, &field.role, defs);
                    let rust_field_name = rust_field_name(&field.name, true);
                    if field.optional {
                        rust_fields.push((rust_field_name, Rust::Option(Box::new(rust_role))));
                    } else {
                        rust_fields.push((rust_field_name, rust_role));
                    }
                }

                defs.push(Definition(name.clone(), Rust::Struct(rust_fields)));
                Rust::Complex(name)
            }

            Asn::SequenceOf(asn) => {
                let name = rust_struct_or_enum_name(name);

                let inner = Rust::Vec(Box::new(Self::unfold_asn_sequence_of_to_rust_vec(
                    &name, asn, defs,
                )));
                defs.push(Definition(name.clone(), Rust::TupleStruct(Box::new(inner))));

                Rust::Complex(name)
            }

            Asn::Choice(entries) => {
                let name = rust_struct_or_enum_name(name);
                let mut rust_entries = Vec::with_capacity(entries.len());

                for ChoiceEntry(entry_name, asn) in entries.iter() {
                    let rust_name = format!("{}{}", name, rust_struct_or_enum_name(entry_name));
                    let rust_role = Self::unfold_asn_sequence_of_to_rust_vec(&rust_name, asn, defs);
                    let rust_field_name = rust_field_name(entry_name, true);
                    rust_entries.push((rust_field_name, rust_role));
                }

                defs.push(Definition(name.clone(), Rust::DataEnum(rust_entries)));
                Rust::Complex(name)
            }

            Asn::Enumerated(variants) => {
                let name = rust_struct_or_enum_name(name);
                let mut rust_variants = Vec::with_capacity(variants.len());

                for variant in variants.iter() {
                    rust_variants.push(rust_variant_name(variant));
                }

                defs.push(Definition(name.clone(), Rust::Enum(rust_variants)));
                Rust::Complex(name)
            }
        }
    }

    fn unfold_asn_sequence_of_to_rust_vec(
        name: &str,
        asn: &Asn,
        defs: &mut Vec<Definition<Rust>>,
    ) -> Rust {
        if let Asn::SequenceOf(asn) = asn {
            Rust::Vec(Box::new(Self::unfold_asn_sequence_of_to_rust_vec(
                name, asn, defs,
            )))
        } else {
            Model::definition_to_rust(&name, asn, defs)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use parser::Parser;

    const SIMPLE_INTEGER_STRUCT_ASN: &str = r"
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
        let model =
            Model::try_from(Parser::new().parse(SIMPLE_INTEGER_STRUCT_ASN).unwrap()).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(1, model.definitions.len());
        assert_eq!(
            Definition(
                "Simple".into(),
                Asn::Sequence(vec![
                    Field {
                        name: "small".into(),
                        role: Asn::Integer(Some(Range(0, 255))),
                        optional: false,
                    },
                    Field {
                        name: "bigger".into(),
                        role: Asn::Integer(Some(Range(0, 65535))),
                        optional: false,
                    },
                    Field {
                        name: "negative".into(),
                        role: Asn::Integer(Some(Range(-1, 255))),
                        optional: false,
                    },
                    Field {
                        name: "unlimited".into(),
                        role: Asn::Integer(None),
                        optional: true,
                    }
                ])
            ),
            model.definitions[0]
        );
    }

    #[test]
    fn test_simple_asn_sequence_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Parser::new().parse(SIMPLE_INTEGER_STRUCT_ASN).unwrap())
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "Simple".into(),
                Rust::Struct(vec![
                    ("small".into(), Rust::U8(Range(0, 255))),
                    ("bigger".into(), Rust::U16(Range(0, 65535))),
                    ("negative".into(), Rust::I16(Range(-1, 255))),
                    ("unlimited".into(), Rust::Option(Box::new(Rust::U64(None)))),
                ])
            ),
            model_rust.definitions[0]
        );
    }

    const INLINE_ASN_WITH_ENUM: &str = r"
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
        let model = Model::try_from(Parser::new().parse(INLINE_ASN_WITH_ENUM).unwrap()).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(1, model.definitions.len());
        assert_eq!(
            Definition(
                "Woah".into(),
                Asn::Sequence(vec![Field {
                    name: "decision".into(),
                    role: Asn::Enumerated(vec![
                        "ABORT".into(),
                        "RETURN".into(),
                        "CONFIRM".into(),
                        "MAYDAY".into(),
                        "THE_CAKE_IS_A_LIE".into()
                    ]),
                    optional: true,
                }])
            ),
            model.definitions[0]
        );
    }

    #[test]
    fn test_inline_asn_enumerated_represented_correctly_as_rust_model() {
        let modle_rust = Model::try_from(Parser::new().parse(INLINE_ASN_WITH_ENUM).unwrap())
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", modle_rust.name);
        assert_eq!(true, modle_rust.imports.is_empty());
        assert_eq!(2, modle_rust.definitions.len());
        assert_eq!(
            Definition(
                "WoahDecision".into(),
                Rust::Enum(vec![
                    "ABORT".into(),
                    "RETURN".into(),
                    "CONFIRM".into(),
                    "MAYDAY".into(),
                    "THE_CAKE_IS_A_LIE".into()
                ])
            ),
            modle_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::Struct(vec![(
                    "decision".into(),
                    Rust::Option(Box::new(Rust::Complex("WoahDecision".into())))
                )])
            ),
            modle_rust.definitions[1]
        );
    }

    const INLINE_ASN_WITH_SEQUENCE_OF: &str = r"
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
            Model::try_from(Parser::new().parse(INLINE_ASN_WITH_SEQUENCE_OF).unwrap()).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(3, model.definitions.len());
        assert_eq!(
            Definition(
                "Ones".into(),
                Asn::SequenceOf(Box::new(Asn::Integer(Some(Range(0, 1)))))
            ),
            model.definitions[0]
        );
        assert_eq!(
            Definition(
                "NestedOnes".into(),
                Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::Integer(Some(
                    Range(0, 1)
                ))))))
            ),
            model.definitions[1]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Asn::Sequence(vec![
                    Field {
                        name: "also-ones".into(),
                        role: Asn::SequenceOf(Box::new(Asn::Integer(Some(Range(0, 1))))),
                        optional: false,
                    },
                    Field {
                        name: "nesteds".into(),
                        role: Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::Integer(
                            Some(Range(0, 1))
                        ))))),
                        optional: false,
                    },
                    Field {
                        name: "optionals".into(),
                        role: Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::Integer(
                            None
                        ))))),
                        optional: true,
                    },
                ])
            ),
            model.definitions[2]
        );
    }

    #[test]
    fn test_inline_asn_sequence_of_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Parser::new().parse(INLINE_ASN_WITH_SEQUENCE_OF).unwrap())
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(3, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "Ones".into(),
                Rust::TupleStruct(Box::new(Rust::Vec(Box::new(Rust::U8(Range(0, 1))))))
            ),
            model_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "NestedOnes".into(),
                Rust::TupleStruct(Box::new(Rust::Vec(Box::new(Rust::Vec(Box::new(
                    Rust::U8(Range(0, 1))
                ))))))
            ),
            model_rust.definitions[1]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::Struct(vec![
                    (
                        "also_ones".into(),
                        Rust::Vec(Box::new(Rust::U8(Range(0, 1))))
                    ),
                    (
                        "nesteds".into(),
                        Rust::Vec(Box::new(Rust::Vec(Box::new(Rust::U8(Range(0, 1))))))
                    ),
                    (
                        "optionals".into(),
                        Rust::Option(Box::new(Rust::Vec(Box::new(Rust::Vec(Box::new(
                            Rust::U64(None)
                        ))))))
                    )
                ])
            ),
            model_rust.definitions[2]
        );
    }

    const INLINE_ASN_WITH_CHOICE: &str = r"
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
        let model = Model::try_from(Parser::new().parse(INLINE_ASN_WITH_CHOICE).unwrap()).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(4, model.definitions.len());
        assert_eq!(
            Definition(
                "This".into(),
                Asn::SequenceOf(Box::new(Asn::Integer(Some(Range(0, 1)))))
            ),
            model.definitions[0]
        );
        assert_eq!(
            Definition(
                "That".into(),
                Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::Integer(Some(
                    Range(0, 1)
                ))))))
            ),
            model.definitions[1]
        );
        assert_eq!(
            Definition(
                "Neither".into(),
                Asn::Enumerated(vec!["ABC".into(), "DEF".into(),])
            ),
            model.definitions[2]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Asn::Sequence(vec![Field {
                    name: "decision".into(),
                    role: Asn::Choice(vec![
                        ChoiceEntry("this".into(), Asn::TypeReference("This".into())),
                        ChoiceEntry("that".into(), Asn::TypeReference("That".into())),
                        ChoiceEntry("neither".into(), Asn::TypeReference("Neither".into())),
                    ]),
                    optional: false
                }])
            ),
            model.definitions[3]
        );
    }
    #[test]
    fn test_inline_asn_choice_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Parser::new().parse(INLINE_ASN_WITH_CHOICE).unwrap())
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(5, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "This".into(),
                Rust::TupleStruct(Box::new(Rust::Vec(Box::new(Rust::U8(Range(0, 1))))))
            ),
            model_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "That".into(),
                Rust::TupleStruct(Box::new(Rust::Vec(Box::new(Rust::Vec(Box::new(
                    Rust::U8(Range(0, 1))
                ))))))
            ),
            model_rust.definitions[1]
        );
        assert_eq!(
            Definition(
                "Neither".into(),
                Rust::Enum(vec!["ABC".into(), "DEF".into(),])
            ),
            model_rust.definitions[2]
        );
        assert_eq!(
            Definition(
                "WoahDecision".into(),
                Rust::DataEnum(vec![
                    ("this".into(), Rust::Complex("This".into())),
                    ("that".into(), Rust::Complex("That".into())),
                    ("neither".into(), Rust::Complex("Neither".into())),
                ])
            ),
            model_rust.definitions[3]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::Struct(vec![(
                    "decision".into(),
                    Rust::Complex("WoahDecision".into())
                )])
            ),
            model_rust.definitions[4]
        );
    }

    const INLINE_ASN_WITH_SEQUENCE: &str = r"
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
        let model =
            Model::try_from(Parser::new().parse(INLINE_ASN_WITH_SEQUENCE).unwrap()).unwrap();

        assert_eq!("SimpleSchema", model.name);
        assert_eq!(true, model.imports.is_empty());
        assert_eq!(1, model.definitions.len());
        assert_eq!(
            Definition(
                "Woah".into(),
                Asn::Sequence(vec![Field {
                    name: "complex".into(),
                    role: Asn::Sequence(vec![
                        Field {
                            name: "ones".into(),
                            role: Asn::Integer(Some(Range(0, 1))),
                            optional: false,
                        },
                        Field {
                            name: "list-ones".into(),
                            role: Asn::SequenceOf(Box::new(Asn::Integer(Some(Range(0, 1))))),
                            optional: false,
                        },
                        Field {
                            name: "optional-ones".into(),
                            role: Asn::SequenceOf(Box::new(Asn::Integer(Some(Range(0, 1))))),
                            optional: true,
                        },
                    ]),
                    optional: true,
                }])
            ),
            model.definitions[0]
        );
    }

    #[test]
    fn test_inline_asn_sequence_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Parser::new().parse(INLINE_ASN_WITH_SEQUENCE).unwrap())
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(2, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "WoahComplex".into(),
                Rust::Struct(vec![
                    ("ones".into(), Rust::U8(Range(0, 1))),
                    (
                        "list_ones".into(),
                        Rust::Vec(Box::new(Rust::U8(Range(0, 1))))
                    ),
                    (
                        "optional_ones".into(),
                        Rust::Option(Box::new(Rust::Vec(Box::new(Rust::U8(Range(0, 1))))))
                    ),
                ])
            ),
            model_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::Struct(vec![(
                    "complex".into(),
                    Rust::Option(Box::new(Rust::Complex("WoahComplex".into())))
                )])
            ),
            model_rust.definitions[1]
        );
    }

    #[test]
    fn test_simple_enum() {
        let mut model_asn = Model::default();
        model_asn.definitions.push(Definition(
            "SimpleEnumTest".into(),
            Asn::Enumerated(vec![
                "Bernd".into(),
                "Das-Verdammte".into(),
                "Brooot".into(),
            ]),
        ));

        let model_rust = model_asn.to_rust();

        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "SimpleEnumTest".into(),
                Rust::Enum(vec!["Bernd".into(), "DasVerdammte".into(), "Brooot".into(),])
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_choice_simple() {
        let mut model_asn = Model::default();
        model_asn.definitions.push(Definition(
            "SimpleChoiceTest".into(),
            Asn::Choice(vec![
                ChoiceEntry("bernd-das-brot".into(), Asn::UTF8String),
                ChoiceEntry("nochSoEinBrot".into(), Asn::OctetString),
            ]),
        ));

        let model_rust = model_asn.to_rust();

        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "SimpleChoiceTest".into(),
                Rust::DataEnum(vec![
                    ("bernd_das_brot".into(), Rust::String),
                    ("noch_so_ein_brot".into(), Rust::VecU8),
                ])
            ),
            model_rust.definitions[0]
        )
    }

    #[test]
    fn test_choice_list_and_nested_list() {
        let mut model_asn = Model::default();
        model_asn.definitions.push(Definition(
            "ListChoiceTestWithNestedList".into(),
            Asn::Choice(vec![
                ChoiceEntry(
                    "normal-List".into(),
                    Asn::SequenceOf(Box::new(Asn::UTF8String)),
                ),
                ChoiceEntry(
                    "NESTEDList".into(),
                    Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::OctetString)))),
                ),
            ]),
        ));

        let model_rust = model_asn.to_rust();

        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "ListChoiceTestWithNestedList".into(),
                Rust::DataEnum(vec![
                    ("normal_list".into(), Rust::Vec(Box::new(Rust::String))),
                    (
                        "nested_list".into(),
                        Rust::Vec(Box::new(Rust::Vec(Box::new(Rust::VecU8))))
                    ),
                ])
            ),
            model_rust.definitions[0]
        )
    }

    #[test]
    fn test_tuple_list() {
        let mut model_asn = Model::default();
        model_asn.name = "TupleTestModel".into();
        model_asn.definitions.push(Definition(
            "TupleTest".into(),
            Asn::SequenceOf(Box::new(Asn::UTF8String)),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("tuple_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "TupleTest".into(),
                Rust::TupleStruct(Box::new(Rust::Vec(Box::new(Rust::String))))
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_nested_tuple_list() {
        let mut model_asn = Model::default();
        model_asn.name = "TupleTestModel".into();
        model_asn.definitions.push(Definition(
            "NestedTupleTest".into(),
            Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::UTF8String)))),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("tuple_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "NestedTupleTest".into(),
                Rust::TupleStruct(Box::new(Rust::Vec(Box::new(Rust::Vec(Box::new(
                    Rust::String
                ))))))
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_optional_list_in_struct() {
        let mut model_asn = Model::default();
        model_asn.name = "OptionalStructListTestModel".into();
        model_asn.definitions.push(Definition(
            "OptionalStructListTest".into(),
            Asn::Sequence(vec![Field {
                name: "strings".into(),
                role: Asn::SequenceOf(Box::new(Asn::UTF8String)),
                optional: true,
            }]),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("optional_struct_list_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "OptionalStructListTest".into(),
                Rust::Struct(vec![(
                    "strings".into(),
                    Rust::Option(Box::new(Rust::Vec(Box::new(Rust::String))))
                )])
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_list_in_struct() {
        let mut model_asn = Model::default();
        model_asn.name = "StructListTestModel".into();
        model_asn.definitions.push(Definition(
            "StructListTest".into(),
            Asn::Sequence(vec![Field {
                name: "strings".into(),
                role: Asn::SequenceOf(Box::new(Asn::UTF8String)),
                optional: false,
            }]),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("struct_list_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "StructListTest".into(),
                Rust::Struct(vec![("strings".into(), Rust::Vec(Box::new(Rust::String)))])
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_nested_list_in_struct() {
        let mut model_asn = Model::default();
        model_asn.name = "NestedStructListTestModel".into();
        model_asn.definitions.push(Definition(
            "NestedStructListTest".into(),
            Asn::Sequence(vec![Field {
                name: "strings".into(),
                role: Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::UTF8String)))),
                optional: false,
            }]),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("nested_struct_list_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "NestedStructListTest".into(),
                Rust::Struct(vec![(
                    "strings".into(),
                    Rust::Vec(Box::new(Rust::Vec(Box::new(Rust::String))))
                )])
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_nice_names() {
        assert_eq!("simple_test", Model::new("SimpleTest").to_rust().name);
        assert_eq!("simple_test", Model::new("SIMPLE_Test").to_rust().name);
        assert_eq!("dry", Model::new("DRY_Module").to_rust().name);
        assert_eq!("dry", Model::new("DRYModule").to_rust().name);
    }
}

#[derive(Debug, Default, Clone, PartialOrd, PartialEq)]
pub struct Import {
    pub what: Vec<String>,
    pub from: String,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ChoiceEntry(String, Asn);

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub struct Range<T>(T, T);

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Definition<T>(String, T);

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Field<T> {
    pub name: String,
    pub role: T,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Asn {
    Boolean,
    Integer(Option<Range<i64>>),
    UTF8String,
    OctetString,

    SequenceOf(Box<Asn>),
    Sequence(Vec<Field<Asn>>),
    Enumerated(Vec<String>),
    Choice(Vec<ChoiceEntry>),
    TypeReference(String),
}

const I8_MAX: i64 = ::std::i8::MAX as i64;
const I16_MAX: i64 = ::std::i16::MAX as i64;
const I32_MAX: i64 = ::std::i32::MAX as i64;
const I64_MAX: i64 = ::std::i64::MAX as i64;

const U8_MAX: u64 = ::std::u8::MAX as u64;
const U16_MAX: u64 = ::std::u16::MAX as u64;
const U32_MAX: u64 = ::std::u32::MAX as u64;
const U64_MAX: u64 = ::std::u64::MAX as u64;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Rust {
    Bool,
    U8(Range<u8>),
    I8(Range<i8>),
    U16(Range<u16>),
    I16(Range<i16>),
    U32(Range<u32>),
    I32(Range<i32>),
    U64(Option<Range<u64>>),
    I64(Range<i64>),
    String,
    VecU8,
    Vec(Box<Rust>),
    Option(Box<Rust>),
    Struct(Vec<(String, Rust)>),
    Enum(Vec<String>),
    DataEnum(Vec<(String, Rust)>),
    TupleStruct(Box<Rust>),
    /// Indicates a complex, custom type that is
    /// not one of rusts known types
    Complex(String),
}

impl Rust {
    pub fn is_primitive(&self) -> bool {
        match self {
            Rust::Bool => true,
            Rust::U8(_) => true,
            Rust::I8(_) => true,
            Rust::U16(_) => true,
            Rust::I16(_) => true,
            Rust::U32(_) => true,
            Rust::I32(_) => true,
            Rust::U64(_) => true,
            Rust::I64(_) => true,
            Rust::String => false,
            _ => false,
        }
    }
}
/*
impl From<ProtobufType> for Rust {
    fn from(proto: ProtobufType) -> Self {
        match proto {
            ProtobufType::Bool => Rust::Bool,
            ProtobufType::SFixed32 => Rust::I32,
            ProtobufType::SFixed64 => Rust::I64,
            ProtobufType::UInt32 => Rust::U32,
            ProtobufType::UInt64 => Rust::U64,
            ProtobufType::SInt32 => Rust::I32,
            ProtobufType::SInt64 => Rust::I64,
            ProtobufType::String => Rust::String,
            ProtobufType::Bytes => Rust::VecU8,
            ProtobufType::Complex(name) => Rust::Complex(name.clone()),
        }
    }
}*/

impl ToString for Rust {
    fn to_string(&self) -> String {
        match self {
            Rust::Bool => "bool",
            Rust::U8(_) => "u8",
            Rust::I8(_) => "i8",
            Rust::U16(_) => "u16",
            Rust::I16(_) => "i16",
            Rust::U32(_) => "u32",
            Rust::I32(_) => "i32",
            Rust::U64(_) => "u64",
            Rust::I64(_) => "i64",
            Rust::String => "String",
            Rust::VecU8 => "Vec<u8>",
            Rust::Vec(inner) => return format!("Vec<{}>", inner.to_string()),
            Rust::Option(inner) => return format!("Option<{}>", inner.to_string()),
            Rust::Struct(_) => unimplemented!(),      // TODO
            Rust::TupleStruct(_) => unimplemented!(), // TODO
            Rust::Enum(_) => unimplemented!(),        // TODO
            Rust::DataEnum(_) => unimplemented!(),    // TODO
            Rust::Complex(name) => return name.clone(),
        }.into()
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum ProtobufType {
    Bool,
    SFixed32,
    SFixed64,
    UInt32,
    UInt64,
    SInt32,
    SInt64,
    String,
    Bytes,
    /// Indicates a complex, custom type that is
    /// not one of rusts known types
    Complex(String),
}

impl ProtobufType {
    pub fn is_primitive(&self) -> bool {
        if let ProtobufType::Complex(_) = self {
            false
        } else {
            true
        }
    }
}

/*
impl From<Rust> for ProtobufType {
    fn from(rust: Rust) -> Self {
        match rust {
            Rust::Bool => ProtobufType::Bool,
            Rust::U8(_) => ProtobufType::UInt32,
            Rust::I8(_) => ProtobufType::SInt32,
            Rust::U16(_) => ProtobufType::UInt32,
            Rust::I16(_) => ProtobufType::SInt32,
            Rust::U32(_) => ProtobufType::UInt32,
            Rust::I32(_) => ProtobufType::SInt32,
            Rust::U64(_) => ProtobufType::UInt64,
            Rust::I64(_) => ProtobufType::SInt64,
            Rust::String => ProtobufType::String,
            Rust::VecU8 => ProtobufType::Bytes,
            Rust::Complex(name) => ProtobufType::Complex(name.clone()),
        }
    }
}*/

impl ToString for ProtobufType {
    fn to_string(&self) -> String {
        match self {
            ProtobufType::Bool => "bool",
            ProtobufType::SFixed32 => "sfixed32",
            ProtobufType::SFixed64 => "sfixed64",
            ProtobufType::UInt32 => "uint32",
            ProtobufType::UInt64 => "uint64",
            ProtobufType::SInt32 => "sint32",
            ProtobufType::SInt64 => "sint64",
            ProtobufType::String => "string",
            ProtobufType::Bytes => "bytes",
            ProtobufType::Complex(name) => return name.clone(),
        }.into()
    }
}

const KEYWORDS: [&str; 9] = [
    "use", "mod", "const", "type", "pub", "enum", "struct", "impl", "trait",
];

pub fn rust_field_name(name: &str, check_for_keywords: bool) -> String {
    let mut name = rust_module_name(name);
    if check_for_keywords {
        for keyword in KEYWORDS.iter() {
            if keyword.eq(&name) {
                name.push_str("_");
                return name;
            }
        }
    }
    name
}

pub fn rust_variant_name(name: &str) -> String {
    let mut out = String::new();
    let mut next_upper = true;
    for c in name.chars() {
        if next_upper {
            out.push_str(&c.to_uppercase().to_string());
            next_upper = false;
        } else if c == '-' {
            next_upper = true;
        } else {
            out.push(c);
        }
    }
    out
}

pub fn rust_struct_or_enum_name(name: &str) -> String {
    rust_variant_name(name)
}

pub fn rust_module_name(name: &str) -> String {
    let mut out = String::new();
    let mut prev_lowered = false;
    let mut prev_alphabetic = false;
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        let mut lowered = false;
        let mut alphabetic = c.is_alphabetic();
        if c.is_uppercase() {
            if !out.is_empty() && prev_alphabetic {
                if !prev_lowered {
                    out.push('_');
                } else if let Some(next) = chars.peek() {
                    if next.is_lowercase() {
                        out.push('_');
                    }
                }
            }
            lowered = true;
            out.push_str(&c.to_lowercase().to_string());
        } else if c == '-' {
            out.push('_');
        } else {
            out.push(c);
        }
        prev_lowered = lowered;
        prev_alphabetic = alphabetic;
    }
    out
}
