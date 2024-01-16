#[cfg(feature = "protobuf")]
pub mod protobuf;
pub mod rust;

pub mod err;
pub mod lit_or_ref;
pub mod parse;

#[cfg(feature = "protobuf")]
pub use self::protobuf::Protobuf;
#[cfg(feature = "protobuf")]
pub use self::protobuf::ProtobufType;
pub use self::rust::Rust;
pub use self::rust::RustType;

use crate::asn::ObjectIdentifier;
use std::fmt::Debug;

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

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct ValueReference<T> {
    pub name: String,
    pub role: T,
    pub value: LiteralValue,
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
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

#[derive(Debug, Default, Clone, PartialOrd, PartialEq, Eq)]
pub struct Import {
    pub what: Vec<String>,
    pub from: String,
    pub from_oid: Option<ObjectIdentifier>,
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct Field<T> {
    pub name: String,
    pub role: T,
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::asn::ObjectIdentifierComponent;
    use crate::asn::{BitString, Choice, ChoiceVariant, Enumerated, EnumeratedVariant, Integer};
    use crate::asn::{Charset, Range, TagProperty};
    use crate::asn::{Size, Tag, Type};
    use crate::model::err::Error;
    use crate::model::lit_or_ref::Resolved;
    use crate::parser::{Location, Token, Tokenizer};

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
