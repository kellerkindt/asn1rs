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

pub enum Error {
    ExpectedText(Backtrace, Token),
    ExpectedTextGot(Backtrace, String, Token),
    ExpectedSeparator(Backtrace, Token),
    ExpectedSeparatorGot(Backtrace, char, Token),
    UnexpectedToken(Backtrace, Token),
    MissingModuleName,
    UnexpectedEndOfStream(Backtrace),
    InvalidRangeValue(Backtrace, Token),
    InvalidNumberForEnumVariant(Backtrace, Token),
}

impl Error {
    pub fn invalid_number_for_enum_variant(token: Token) -> Self {
        Error::InvalidNumberForEnumVariant(Backtrace::new(), token)
    }

    pub fn invalid_range_value(token: Token) -> Self {
        Error::InvalidRangeValue(Backtrace::new(), token)
    }

    pub fn no_text(token: Token) -> Self {
        Error::ExpectedText(Backtrace::new(), token)
    }

    pub fn expected_text(text: String, token: Token) -> Self {
        Error::ExpectedTextGot(Backtrace::new(), text, token)
    }

    pub fn no_separator(token: Token) -> Self {
        Error::ExpectedSeparator(Backtrace::new(), token)
    }

    pub fn expected_separator(separator: char, token: Token) -> Self {
        Error::ExpectedSeparatorGot(Backtrace::new(), separator, token)
    }

    pub fn unexpected_token(token: Token) -> Self {
        Error::UnexpectedToken(Backtrace::new(), token)
    }

    pub fn unexpected_end_of_stream() -> Self {
        Error::UnexpectedEndOfStream(Backtrace::new())
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            Error::ExpectedText(bt, _) => Some(bt),
            Error::ExpectedTextGot(bt, _, _) => Some(bt),
            Error::ExpectedSeparator(bt, _) => Some(bt),
            Error::ExpectedSeparatorGot(bt, _, _) => Some(bt),
            Error::UnexpectedToken(bt, _) => Some(bt),
            Error::MissingModuleName => None,
            Error::UnexpectedEndOfStream(bt) => Some(bt),
            Error::InvalidRangeValue(bt, _) => Some(bt),
            Error::InvalidNumberForEnumVariant(bt, _) => Some(bt),
        }
    }
}

impl StdError for Error {}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "{}", self)?;
        if let Some(bt) = self.backtrace() {
            writeln!(f, "{:?}", bt)?;
        }
        Ok(())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Error::ExpectedText(_, token) => write!(
                f,
                "At line {}, column {} expected text, but instead got: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            Error::ExpectedTextGot(_, text, token) => write!(
                f,
                "At line {}, column {} expected a text like \"{}\", but instead got: {}",
                token.location().line(),
                token.location().column(),
                text,
                token,
            ),
            Error::ExpectedSeparator(_, token) => write!(
                f,
                "At line {}, column {} expected separator, but instead got: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            Error::ExpectedSeparatorGot(_, separator, token) => write!(
                f,
                "At line {}, column {} expected a separator like '{}', but instead got: {}",
                token.location().line(),
                token.location().column(),
                separator,
                token,
            ),
            Error::UnexpectedToken(_, token) => write!(
                f,
                "At line {}, column {} an unexpected token was encountered: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            Error::MissingModuleName => {
                writeln!(f, "The ASN definition is missing the module name")
            }
            Error::UnexpectedEndOfStream(_) => write!(f, "Unexpected end of stream or file"),
            Error::InvalidRangeValue(_, token) => write!(
                f,
                "At line {}, column {} an unexpected range value was encountered: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
            Error::InvalidNumberForEnumVariant(_, token) => write!(
                f,
                "At line {}, column {} an invalid value for an enum variant was encountered: {}",
                token.location().line(),
                token.location().column(),
                token,
            ),
        }
    }
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
    pub fn try_from(value: Vec<Token>) -> Result<Self, Error> {
        let mut model = Model::default();
        let mut iter = value.into_iter();

        model.name = Self::read_name(&mut iter)?;
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

    fn read_name(iter: &mut IntoIter<Token>) -> Result<String, Error> {
        iter.next()
            .and_then(|token| token.into_text())
            .ok_or(Error::MissingModuleName)
    }

    fn skip_until_after_text_ignore_ascii_case(
        iter: &mut IntoIter<Token>,
        text: &str,
    ) -> Result<(), Error> {
        for t in iter {
            if t.eq_text_ignore_ascii_case(text) {
                return Ok(());
            }
        }
        Err(Error::unexpected_end_of_stream())
    }

    fn read_imports(iter: &mut IntoIter<Token>) -> Result<Vec<Import>, Error> {
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
                    imports.push(import);
                    import = Import::default();
                }
            }
        }
        Err(Error::unexpected_end_of_stream())
    }
    fn read_definition(iter: &mut IntoIter<Token>, name: String) -> Result<Definition<Asn>, Error> {
        Self::next_separator_ignore_case(iter, ':')?;
        Self::next_separator_ignore_case(iter, ':')?;
        Self::next_separator_ignore_case(iter, '=')?;

        let token = Self::next(iter)?;

        if token.text().map_or(false, |s| s.eq("SEQUENCE")) {
            Ok(Definition(name, Self::read_sequence_or_sequence_of(iter)?))
        } else if token
            .text()
            .map_or(false, |s| s.eq_ignore_ascii_case("ENUMERATED"))
        {
            Ok(Definition(
                name,
                Asn::Enumerated(Enumerated::try_from(iter)?),
            ))
        } else if token
            .text()
            .map_or(false, |s| s.eq_ignore_ascii_case("CHOICE"))
        {
            Ok(Definition(name, Asn::Choice(Self::read_choice(iter)?)))
        } else if let Some(text) = token.text() {
            Ok(Definition(
                name,
                Self::read_role_given_text(iter, text.to_string())?,
            ))
        } else {
            Err(Error::unexpected_token(token))
        }
    }

    fn read_role(iter: &mut IntoIter<Token>) -> Result<Asn, Error> {
        let text = Self::next_text(iter)?;
        Self::read_role_given_text(iter, text)
    }

    fn read_role_given_text(iter: &mut IntoIter<Token>, text: String) -> Result<Asn, Error> {
        if text.eq_ignore_ascii_case("INTEGER") {
            Self::next_separator_ignore_case(iter, '(')?;
            let start = Self::next(iter)?;
            Self::next_separator_ignore_case(iter, '.')?;
            Self::next_separator_ignore_case(iter, '.')?;
            let end = Self::next(iter)?;
            Self::next_separator_ignore_case(iter, ')')?;
            if start.eq_text("0") && end.eq_text_ignore_ascii_case("MAX") {
                Ok(Asn::Integer(None))
            } else {
                Ok(Asn::Integer(Some(Range(
                    start
                        .text()
                        .and_then(|t| t.parse::<i64>().ok())
                        .ok_or_else(|| Error::invalid_range_value(start))?,
                    end.text()
                        .and_then(|t| t.parse::<i64>().ok())
                        .ok_or_else(|| Error::invalid_range_value(end))?,
                ))))
            }
        } else if text.eq_ignore_ascii_case("BOOLEAN") {
            Ok(Asn::Boolean)
        } else if text.eq_ignore_ascii_case("UTF8String") {
            Ok(Asn::UTF8String)
        } else if text.eq_ignore_ascii_case("OCTET") {
            let token = Self::next(iter)?;
            if token.text().map_or(false, |t| t.eq("STRING")) {
                Ok(Asn::OctetString)
            } else {
                Err(Error::unexpected_token(token))
            }
        } else if text.eq_ignore_ascii_case("CHOICE") {
            Ok(Asn::Choice(Self::read_choice(iter)?))
        } else if text.eq_ignore_ascii_case("ENUMERATED") {
            Ok(Asn::Enumerated(Enumerated::try_from(iter)?))
        } else if text.eq_ignore_ascii_case("SEQUENCE") {
            Ok(Self::read_sequence_or_sequence_of(iter)?)
        } else {
            Ok(Asn::TypeReference(text))
        }
    }

    fn read_sequence_or_sequence_of(iter: &mut IntoIter<Token>) -> Result<Asn, Error> {
        let token = Self::next(iter)?;

        if token.eq_text_ignore_ascii_case("OF") {
            Ok(Asn::SequenceOf(Box::new(Self::read_role(iter)?)))
        } else if token.eq_separator('{') {
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
            Err(Error::unexpected_token(token))
        }
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
        let mut token = Self::next(iter)?;
        if let Some(_optional_flag) = token.text().map(|s| s.eq_ignore_ascii_case("OPTIONAL")) {
            field.optional = true;
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

    fn next(iter: &mut IntoIter<Token>) -> Result<Token, Error> {
        iter.next().ok_or_else(Error::unexpected_end_of_stream)
    }

    fn next_text(iter: &mut IntoIter<Token>) -> Result<String, Error> {
        Self::next(iter)?.into_text_or_else(Error::no_text)
    }

    fn next_separator_ignore_case(
        iter: &mut IntoIter<Token>,
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::parser::Tokenizer;

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
                ]),
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
                Asn::Sequence(vec![Field {
                    name: "decision".into(),
                    role: Asn::Enumerated(Enumerated::from_names(
                        ["ABORT", "RETURN", "CONFIRM", "MAYDAY", "THE_CAKE_IS_A_LIE",].iter()
                    )),
                    optional: true,
                }]),
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
                Asn::SequenceOf(Box::new(Asn::Integer(Some(Range(0, 1))))),
            ),
            model.definitions[0]
        );
        assert_eq!(
            Definition(
                "NestedOnes".into(),
                Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::Integer(Some(
                    Range(0, 1)
                )))))),
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
                ]),
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
                Asn::SequenceOf(Box::new(Asn::Integer(Some(Range(0, 1))))),
            ),
            model.definitions[0]
        );
        assert_eq!(
            Definition(
                "That".into(),
                Asn::SequenceOf(Box::new(Asn::SequenceOf(Box::new(Asn::Integer(Some(
                    Range(0, 1)
                )))))),
            ),
            model.definitions[1]
        );
        assert_eq!(
            Definition(
                "Neither".into(),
                Asn::Enumerated(Enumerated::from_names(["ABC".into(), "DEF".into()].iter())),
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
                    optional: false,
                }]),
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
                }]),
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
                Asn::Integer(Some(Range(0, 65_535))),
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
            &[Definition("SimpleStringType".to_string(), Asn::UTF8String)][..],
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
                    Asn::Enumerated(Enumerated::from_names(["abc", "def"].iter())),
                ),
                Definition(
                    "WithExplicitNumber".to_string(),
                    Asn::Enumerated(Enumerated {
                        variants: vec![
                            EnumeratedVariant {
                                name: "abc".to_string(),
                                number: Some(1)
                            },
                            EnumeratedVariant {
                                name: "def".to_string(),
                                number: Some(9)
                            }
                        ],
                        default: None,
                    }),
                ),
                Definition(
                    "WithExplicitNumberAndDefaultMark".to_string(),
                    Asn::Enumerated(Enumerated {
                        variants: vec![
                            EnumeratedVariant {
                                name: "abc".to_string(),
                                number: Some(4)
                            },
                            EnumeratedVariant {
                                name: "def".to_string(),
                                number: Some(7)
                            },
                        ],
                        default: Some(1),
                    }),
                ),
                Definition(
                    "WithExplicitNumberAndDefaultMarkV2".to_string(),
                    Asn::Enumerated(Enumerated {
                        variants: vec![
                            EnumeratedVariant {
                                name: "abc".to_string(),
                                number: Some(8)
                            },
                            EnumeratedVariant {
                                name: "def".to_string(),
                                number: Some(1)
                            },
                            EnumeratedVariant {
                                name: "v2".to_string(),
                                number: Some(11)
                            }
                        ],
                        default: Some(1),
                    }),
                )
            ][..],
            &model.definitions[..]
        )
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
pub struct Range<T>(pub T, pub T);

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Definition<T>(pub String, pub T);

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
    Enumerated(Enumerated),
    Choice(Vec<ChoiceEntry>),
    TypeReference(String),
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Enumerated {
    variants: Vec<EnumeratedVariant>,
    default: Option<usize>,
}

impl Enumerated {
    #[cfg(test)]
    pub(crate) fn from_names<'a, 'b: 'a>(variants: impl Iterator<Item = &'a &'b str>) -> Self {
        Self {
            variants: variants
                .map(|name| EnumeratedVariant::from_name(name))
                .collect(),
            default: None,
        }
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

    pub fn default(&self) -> Option<(usize, &EnumeratedVariant)> {
        match self.default {
            Some(index) if index < self.variants.len() => Some((index, &self.variants[index])),
            _ => None,
        }
    }
}

impl TryFrom<&mut IntoIter<Token>> for Enumerated {
    type Error = Error;

    fn try_from(iter: &mut IntoIter<Token>) -> Result<Self, Self::Error> {
        Model::<Asn>::next_separator_ignore_case(iter, '{')?;
        let mut enumerated = Self {
            variants: Vec::new(),
            default: None,
        };

        loop {
            let token = Model::<Asn>::next(iter)?;

            if token.eq_separator('.') && !enumerated.variants.is_empty() {
                Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                enumerated.default = Some(enumerated.variants.len() - 1);
                loop_ctrl_separator!(Model::<Asn>::next(iter)?);
            } else {
                let variant_name = token.into_text_or_else(Error::no_text)?;
                let token = Model::<Asn>::next(iter)?;

                if token.eq_separator(',') || token.eq_separator('}') {
                    enumerated.variants.push(EnumeratedVariant {
                        name: variant_name,
                        number: None,
                    });
                    loop_ctrl_separator!(token);
                } else if token.eq_separator('(') {
                    let token = Model::<Asn>::next(iter)?;
                    let number = token
                        .text()
                        .and_then(|t| t.parse::<usize>().ok())
                        .ok_or_else(|| Error::invalid_number_for_enum_variant(token))?;
                    Model::<Asn>::next_separator_ignore_case(iter, ')')?;
                    enumerated.variants.push(EnumeratedVariant {
                        name: variant_name,
                        number: Some(number),
                    });
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
    name: String,
    number: Option<usize>,
}

impl EnumeratedVariant {
    #[cfg(test)]
    pub(crate) fn from_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
            number: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn number(&self) -> Option<usize> {
        self.number
    }
}
