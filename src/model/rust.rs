use model::Asn;
use model::ChoiceEntry;
use model::Definition;
use model::Import;
use model::Model;
use model::Range;

const I8_MAX: i64 = ::std::i8::MAX as i64;
const I16_MAX: i64 = ::std::i16::MAX as i64;
const I32_MAX: i64 = ::std::i32::MAX as i64;
//const I64_MAX: i64 = ::std::i64::MAX as i64;

const U8_MAX: u64 = ::std::u8::MAX as u64;
const U16_MAX: u64 = ::std::u16::MAX as u64;
const U32_MAX: u64 = ::std::u32::MAX as u64;
//const U64_MAX: u64 = ::std::u64::MAX as u64;

/// Integers are ordered where Ixx < Uxx so
/// that when comparing two instances `RustType`
/// and a > b, then the integer type of a can
/// use ::from(..) to cast from b
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum RustType {
    Bool,
    I8(Range<i8>),
    U8(Range<u8>),
    I16(Range<i16>),
    U16(Range<u16>),
    I32(Range<i32>),
    U32(Range<u32>),
    I64(Range<i64>),
    U64(Option<Range<u64>>),
    String,
    VecU8,
    Vec(Box<RustType>),
    Option(Box<RustType>),

    /// Indicates a complex, custom type that is
    /// not one of rusts known types. This can be
    /// thought of as a "ReferenceType"; declaring usage,
    /// but not being declared here
    Complex(String),
}

impl RustType {
    pub fn into_inner_type(self) -> RustType {
        if self.is_primitive() {
            return self;
        }
        match self.clone() {
            RustType::Vec(inner) => inner.into_inner_type(),
            RustType::Option(inner) => inner.into_inner_type(),
            _ => self,
        }
    }

    pub fn to_inner(&self) -> Option<String> {
        if self.is_primitive() {
            return Some(self.to_string());
        }
        match self {
            RustType::Vec(inner) => inner.to_inner(),
            RustType::Option(inner) => inner.to_inner(),
            _ => None,
        }
    }

    pub fn to_inner_type_string(&self) -> String {
        self.to_inner().unwrap_or_else(|| self.to_string())
    }

    pub fn no_option(self) -> Self {
        match self {
            RustType::Option(inner) => inner.no_option(),
            rust => rust
        }
    }

    pub fn is_primitive(&self) -> bool {
        match self {
            RustType::Bool => true,
            RustType::U8(_) => true,
            RustType::I8(_) => true,
            RustType::U16(_) => true,
            RustType::I16(_) => true,
            RustType::U32(_) => true,
            RustType::I32(_) => true,
            RustType::U64(_) => true,
            RustType::I64(_) => true,
            _ => false,
        }
    }

    pub fn integer_range_str(&self) -> Option<Range<String>> {
        match self {
            RustType::Bool => None,
            RustType::U8(Range(min, max)) => Some(Range(min.to_string(), max.to_string())),
            RustType::I8(Range(min, max)) => Some(Range(min.to_string(), max.to_string())),
            RustType::U16(Range(min, max)) => Some(Range(min.to_string(), max.to_string())),
            RustType::I16(Range(min, max)) => Some(Range(min.to_string(), max.to_string())),
            RustType::U32(Range(min, max)) => Some(Range(min.to_string(), max.to_string())),
            RustType::I32(Range(min, max)) => Some(Range(min.to_string(), max.to_string())),
            RustType::U64(None) => Some(Range("0".into(), ::std::i64::MAX.to_string())), // i64 max!
            RustType::U64(Some(Range(min, max))) => Some(Range(min.to_string(), max.to_string())),
            RustType::I64(Range(min, max)) => Some(Range(min.to_string(), max.to_string())),
            RustType::String => None,
            RustType::VecU8 => None,
            RustType::Vec(inner) => inner.integer_range_str(),
            RustType::Option(inner) => inner.integer_range_str(),
            RustType::Complex(_) => None,
        }
    }

    pub fn similar(&self, other: &Self) -> bool {
        match self {
            RustType::Bool => return *other == RustType::Bool,
            RustType::U8(_) => if let RustType::U8(_) = other {
                return true;
            },
            RustType::I8(_) => if let RustType::I8(_) = other {
                return true;
            },
            RustType::U16(_) => if let RustType::U16(_) = other {
                return true;
            },
            RustType::I16(_) => if let RustType::I16(_) = other {
                return true;
            },
            RustType::U32(_) => if let RustType::U32(_) = other {
                return true;
            },
            RustType::I32(_) => if let RustType::I32(_) = other {
                return true;
            },
            RustType::U64(_) => if let RustType::U64(_) = other {
                return true;
            },
            RustType::I64(_) => if let RustType::I64(_) = other {
                return true;
            },
            RustType::String => if let RustType::String = other {
                return true;
            },
            RustType::VecU8 => if let RustType::VecU8 = other {
                return true;
            },
            RustType::Vec(inner_a) => if let RustType::Vec(inner_b) = other {
                return inner_a.similar(inner_b);
            },
            RustType::Option(inner_a) => if let RustType::Option(inner_b) = other {
                return inner_a.similar(inner_b);
            },
            RustType::Complex(inner_a) => if let RustType::Complex(inner_b) = other {
                return inner_a.eq(inner_b);
            },
        };
        false
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Rust {
    Struct(Vec<(String, RustType)>),
    Enum(Vec<String>),
    DataEnum(Vec<(String, RustType)>),

    /// Used to represent a single, unnamed inner type
    TupleStruct(RustType),
}

impl ToString for RustType {
    fn to_string(&self) -> String {
        match self {
            RustType::Bool => "bool",
            RustType::U8(_) => "u8",
            RustType::I8(_) => "i8",
            RustType::U16(_) => "u16",
            RustType::I16(_) => "i16",
            RustType::U32(_) => "u32",
            RustType::I32(_) => "i32",
            RustType::U64(_) => "u64",
            RustType::I64(_) => "i64",
            RustType::String => "String",
            RustType::VecU8 => "Vec<u8>",
            RustType::Vec(inner) => return format!("Vec<{}>", inner.to_string()),
            RustType::Option(inner) => return format!("Option<{}>", inner.to_string()),
            RustType::Complex(name) => return name.clone(),
        }.into()
    }
}

impl Model<Rust> {
    pub fn convert_asn_to_rust(asn_model: &Model<Asn>) -> Model<Rust> {
        let mut model = Model {
            name: rust_module_name(&asn_model.name),
            imports: asn_model
                .imports
                .iter()
                .map(|i| Import {
                    what: i.what.iter().map(|w| rust_struct_or_enum_name(w)).collect(),
                    from: rust_module_name(&i.from),
                }).collect(),
            definitions: Vec::with_capacity(asn_model.definitions.len()),
        };
        for Definition(name, asn) in &asn_model.definitions {
            let rust_name = rust_struct_or_enum_name(name);
            Self::definition_to_rust(&rust_name, asn, &mut model.definitions);
        }
        model
    }

    /// Converts the given `Asn` value to `Rust`, adding new `Defintion`s as
    /// necessary (inlined types cannot be represented in rust and thus need to
    /// be extracted to their own types).
    /// The returned value is what shall be used to reference to the definition
    /// and can therefore be used to be inserted in the parent element.
    ///
    /// The name is expected in a valid and rusty way
    pub fn definition_to_rust(name: &str, asn: &Asn, defs: &mut Vec<Definition<Rust>>) {
        match asn {
            Asn::Boolean
            | Asn::Integer(_)
            | Asn::UTF8String
            | Asn::OctetString
            | Asn::TypeReference(_) => {
                let rust_type = Self::definition_type_to_rust_type(name, asn, defs);
                defs.push(Definition(name.into(), Rust::TupleStruct(rust_type)));
            }

            Asn::Sequence(fields) => {
                let mut rust_fields = Vec::with_capacity(fields.len());

                for field in fields.iter() {
                    let rust_name = format!("{}{}", name, rust_struct_or_enum_name(&field.name));
                    let rust_role =
                        Self::definition_type_to_rust_type(&rust_name, &field.role, defs);
                    let rust_field_name = rust_field_name(&field.name);
                    if field.optional {
                        rust_fields.push((rust_field_name, RustType::Option(Box::new(rust_role))));
                    } else {
                        rust_fields.push((rust_field_name, rust_role));
                    }
                }

                defs.push(Definition(name.into(), Rust::Struct(rust_fields)));
            }

            Asn::SequenceOf(asn) => {
                let inner = RustType::Vec(Box::new(Self::definition_type_to_rust_type(
                    &name, asn, defs,
                )));
                defs.push(Definition(name.into(), Rust::TupleStruct(inner)));
            }

            Asn::Choice(entries) => {
                let mut rust_entries = Vec::with_capacity(entries.len());

                for ChoiceEntry(entry_name, asn) in entries.iter() {
                    let rust_name = format!("{}{}", name, rust_struct_or_enum_name(entry_name));
                    let rust_role = Self::definition_type_to_rust_type(&rust_name, asn, defs);
                    let rust_field_name = rust_variant_name(entry_name);
                    rust_entries.push((rust_field_name, rust_role));
                }

                defs.push(Definition(name.into(), Rust::DataEnum(rust_entries)));
            }

            Asn::Enumerated(variants) => {
                let mut rust_variants = Vec::with_capacity(variants.len());

                for variant in variants.iter() {
                    rust_variants.push(rust_variant_name(variant));
                }

                defs.push(Definition(name.into(), Rust::Enum(rust_variants)));
            }
        }
    }

    pub fn definition_type_to_rust_type(
        name: &str,
        asn: &Asn,
        defs: &mut Vec<Definition<Rust>>,
    ) -> RustType {
        match asn {
            Asn::Boolean => RustType::Bool,
            Asn::Integer(Some(Range(min, max))) => {
                let min = *min;
                let max = *max;
                if min >= 0 {
                    match max as u64 {
                        m if m <= U8_MAX => RustType::U8(Range(min as u8, max as u8)),
                        m if m <= U16_MAX => RustType::U16(Range(min as u16, max as u16)),
                        m if m <= U32_MAX => RustType::U32(Range(min as u32, max as u32)),
                        _/*m if m <= U64_MAX*/ => RustType::U64(Some(Range(min as u64, max as u64))),
                        //_ => panic!("This should never happen, since max (as u64 frm i64) cannot be greater than U64_MAX")
                    }
                } else {
                    let max_amplitude = (min - 1).abs().max(max);
                    match max_amplitude {
                        _ if max_amplitude <= I8_MAX => RustType::I8(Range(min as i8, max as i8)),
                        _ if max_amplitude <= I16_MAX => RustType::I16(Range(min as i16, max as i16)),
                        _ if max_amplitude <= I32_MAX => RustType::I32(Range(min as i32, max as i32)),
                        _/*if max_amplitude <= I64_MAX*/ => RustType::I64(Range(min as i64, max as i64)),
                        //_ => panic!("This should never happen, since max (being i64) cannot be greater than I64_MAX")
                    }
                }
            }
            Asn::Integer(None) => RustType::U64(None),
            Asn::UTF8String => RustType::String,
            Asn::OctetString => RustType::VecU8,
            Asn::SequenceOf(asn) => RustType::Vec(Box::new(Self::definition_type_to_rust_type(
                name, asn, defs,
            ))),
            Asn::Sequence(_) | Asn::Enumerated(_) | Asn::Choice(_) => {
                let name = rust_struct_or_enum_name(name);
                Self::definition_to_rust(&name, asn, defs);
                RustType::Complex(name)
            }
            Asn::TypeReference(name) => RustType::Complex(name.clone()),
        }
    }
}

pub fn rust_field_name(name: &str) -> String {
    rust_module_name(name)
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

#[cfg(test)]
mod test {
    use super::*;
    use model::test::*;
    use model::Field;
    use parser::Parser;

    #[test]
    fn test_simple_asn_sequence_represented_correctly_as_rust_model() {
        let model_rust =
            Model::try_from(Parser::default().parse(SIMPLE_INTEGER_STRUCT_ASN).unwrap())
                .unwrap()
                .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "Simple".into(),
                Rust::Struct(vec![
                    ("small".into(), RustType::U8(Range(0, 255))),
                    ("bigger".into(), RustType::U16(Range(0, 65535))),
                    ("negative".into(), RustType::I16(Range(-1, 255))),
                    (
                        "unlimited".into(),
                        RustType::Option(Box::new(RustType::U64(None)))
                    ),
                ])
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_inline_asn_enumerated_represented_correctly_as_rust_model() {
        let modle_rust = Model::try_from(Parser::default().parse(INLINE_ASN_WITH_ENUM).unwrap())
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
                    RustType::Option(Box::new(RustType::Complex("WoahDecision".into())))
                )])
            ),
            modle_rust.definitions[1]
        );
    }

    #[test]
    fn test_inline_asn_sequence_of_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(
            Parser::default()
                .parse(INLINE_ASN_WITH_SEQUENCE_OF)
                .unwrap(),
        ).unwrap()
        .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(3, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "Ones".into(),
                Rust::TupleStruct(RustType::Vec(Box::new(RustType::U8(Range(0, 1)))))
            ),
            model_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "NestedOnes".into(),
                Rust::TupleStruct(RustType::Vec(Box::new(RustType::Vec(Box::new(
                    RustType::U8(Range(0, 1))
                )))))
            ),
            model_rust.definitions[1]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::Struct(vec![
                    (
                        "also_ones".into(),
                        RustType::Vec(Box::new(RustType::U8(Range(0, 1))))
                    ),
                    (
                        "nesteds".into(),
                        RustType::Vec(Box::new(RustType::Vec(Box::new(RustType::U8(Range(0, 1))))))
                    ),
                    (
                        "optionals".into(),
                        RustType::Option(Box::new(RustType::Vec(Box::new(RustType::Vec(
                            Box::new(RustType::U64(None))
                        )))))
                    )
                ])
            ),
            model_rust.definitions[2]
        );
    }

    #[test]
    fn test_inline_asn_choice_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Parser::default().parse(INLINE_ASN_WITH_CHOICE).unwrap())
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(5, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "This".into(),
                Rust::TupleStruct(RustType::Vec(Box::new(RustType::U8(Range(0, 1)))))
            ),
            model_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "That".into(),
                Rust::TupleStruct(RustType::Vec(Box::new(RustType::Vec(Box::new(
                    RustType::U8(Range(0, 1))
                )))))
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
                    ("This".into(), RustType::Complex("This".into())),
                    ("That".into(), RustType::Complex("That".into())),
                    ("Neither".into(), RustType::Complex("Neither".into())),
                ])
            ),
            model_rust.definitions[3]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::Struct(vec![(
                    "decision".into(),
                    RustType::Complex("WoahDecision".into())
                )])
            ),
            model_rust.definitions[4]
        );
    }

    #[test]
    fn test_inline_asn_sequence_represented_correctly_as_rust_model() {
        let model_rust =
            Model::try_from(Parser::default().parse(INLINE_ASN_WITH_SEQUENCE).unwrap())
                .unwrap()
                .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(2, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "WoahComplex".into(),
                Rust::Struct(vec![
                    ("ones".into(), RustType::U8(Range(0, 1))),
                    (
                        "list_ones".into(),
                        RustType::Vec(Box::new(RustType::U8(Range(0, 1))))
                    ),
                    (
                        "optional_ones".into(),
                        RustType::Option(Box::new(RustType::Vec(Box::new(RustType::U8(Range(
                            0, 1
                        ))))))
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
                    RustType::Option(Box::new(RustType::Complex("WoahComplex".into())))
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
                    ("BerndDasBrot".into(), RustType::String),
                    ("NochSoEinBrot".into(), RustType::VecU8),
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
                    (
                        "NormalList".into(),
                        RustType::Vec(Box::new(RustType::String))
                    ),
                    (
                        "NESTEDList".into(),
                        RustType::Vec(Box::new(RustType::Vec(Box::new(RustType::VecU8))))
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
                Rust::TupleStruct(RustType::Vec(Box::new(RustType::String)))
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
                Rust::TupleStruct(RustType::Vec(Box::new(RustType::Vec(Box::new(
                    RustType::String
                )))))
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
                    RustType::Option(Box::new(RustType::Vec(Box::new(RustType::String))))
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
                Rust::Struct(vec![(
                    "strings".into(),
                    RustType::Vec(Box::new(RustType::String))
                )])
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
                    RustType::Vec(Box::new(RustType::Vec(Box::new(RustType::String))))
                )])
            ),
            model_rust.definitions[0]
        );
    }

}
