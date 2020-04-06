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
use std::vec::IntoIter;

macro_rules! loop_ctrl_separator {
    ($separator:expr) => {
        match $separator {
            ',' => continue,
            '}' => break,
            separator => {
                return Err(Error::UnexpectedToken(
                    Backtrace::new(),
                    Token::Separator(separator),
                ));
            }
        }
    };
}

#[derive(Debug)]
pub enum Error {
    ExpectedTextGot(Backtrace, String, String),
    ExpectedSeparatorGot(Backtrace, char, char),
    UnexpectedToken(Backtrace, Token),
    MissingModuleName,
    UnexpectedEndOfStream,
    InvalidRangeValue,
    InvalidNumberForEnumVariant(Backtrace, String),
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
        Self::skip_after(&mut iter, &Token::Text("BEGIN".into()))?;

        while let Some(token) = iter.next() {
            match token {
                t @ Token::Separator(_) => return Err(Error::UnexpectedToken(Backtrace::new(), t)),
                Token::Text(text) => {
                    let lower = text.to_lowercase();

                    if lower.eq("end") {
                        model.make_names_nice();
                        return Ok(model);
                    } else if lower.eq("imports") {
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
            })
            .ok_or(Error::MissingModuleName)
    }

    fn skip_after(iter: &mut IntoIter<Token>, token: &Token) -> Result<(), Error> {
        for t in iter {
            if t.eq(token) {
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
                            if s.eq(",") {
                            } else if lower.eq("from") {
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
            Err(Error::UnexpectedToken(Backtrace::new(), token))
        }
    }

    fn read_role(iter: &mut IntoIter<Token>) -> Result<Asn, Error> {
        let text = Self::next_text(iter)?;
        Self::read_role_given_text(iter, text)
    }

    fn read_role_given_text(iter: &mut IntoIter<Token>, text: String) -> Result<Asn, Error> {
        if text.eq_ignore_ascii_case("INTEGER") {
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
        } else if text.eq_ignore_ascii_case("BOOLEAN") {
            Ok(Asn::Boolean)
        } else if text.eq_ignore_ascii_case("UTF8String") {
            Ok(Asn::UTF8String)
        } else if text.eq_ignore_ascii_case("OCTET") {
            let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
            if token.text().map_or(false, |t| t.eq("STRING")) {
                Ok(Asn::OctetString)
            } else {
                Err(Error::UnexpectedToken(Backtrace::new(), token))
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
        let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
        match token {
            Token::Text(of) => {
                if of.eq_ignore_ascii_case("OF") {
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
        if let Some(_optional_flag) = token.text().map(|s| s.eq_ignore_ascii_case("OPTIONAL")) {
            field.optional = true;
            token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
        }

        let (continues, ends) = token
            .separator()
            .map_or((false, false), |s| (s == ',', s == '}'));

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
            Err(Error::ExpectedSeparatorGot(Backtrace::new(), text, token))
        }
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
            r"
            SimpleSchema DEFINITIONS AUTOMATIC TAGS ::=
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
            match iter.next().ok_or(Error::UnexpectedEndOfStream)? {
                Token::Text(variant_name) => match Model::<Asn>::next_seperator(iter)? {
                    s @ ',' | s @ '}' => {
                        enumerated.variants.push(EnumeratedVariant {
                            name: variant_name,
                            number: None,
                        });
                        loop_ctrl_separator!(s);
                    }
                    '(' => {
                        let text = Model::<Asn>::next_text(iter)?;
                        let number = text.parse::<usize>().map_err(|_| {
                            Error::InvalidNumberForEnumVariant(Backtrace::new(), text)
                        })?;
                        Model::<Asn>::next_separator_ignore_case(iter, ')')?;
                        enumerated.variants.push(EnumeratedVariant {
                            name: variant_name,
                            number: Some(number),
                        });
                        loop_ctrl_separator!(Model::<Asn>::next_seperator(iter)?);
                    }
                    separator => loop_ctrl_separator!(separator),
                },
                Token::Separator('.') if !enumerated.variants.is_empty() => {
                    Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                    Model::<Asn>::next_separator_ignore_case(iter, '.')?;
                    enumerated.default = Some(enumerated.variants.len() - 1);
                    loop_ctrl_separator!(Model::<Asn>::next_seperator(iter)?);
                }
                token => return Err(Error::UnexpectedToken(Backtrace::new(), token)),
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
