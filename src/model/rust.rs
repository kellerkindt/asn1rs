use model::*;


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


impl Model<Rust> {
    pub fn convert_asn_to_rust(asn_model: &Model<Asn>) -> Model<Rust> {
        let mut model = Model {
            name: rust_module_name(&asn_model.name),
            imports: asn_model.imports.clone(),
            definitions: Vec::with_capacity(asn_model.definitions.len()),
        };
        for Definition(name, asn) in asn_model.definitions.iter() {
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
    use model::test::*;

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

}