use model::rust::*;
use model::*;

const TUPLE_VARIABLE_NAME_REPLACEMENT: &str = "value";
const DATAENUM_VARIABLE_NAME_REPLACEMENT: &str = "value";

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
    Repeated(Box<ProtobufType>),
    OneOf(Vec<(String, ProtobufType)>),
    /// Indicates a complex, custom type that is
    /// not one of rusts known types
    Complex(String),
}

impl ProtobufType {
    pub fn is_primitive(&self) -> bool {
        match self {
            ProtobufType::Bool => true,
            ProtobufType::SFixed32 => true,
            ProtobufType::SFixed64 => true,
            ProtobufType::UInt32 => true,
            ProtobufType::UInt64 => true,
            ProtobufType::SInt32 => true,
            ProtobufType::SInt64 => true,
            ProtobufType::String => true,
            ProtobufType::Bytes => true,
            ProtobufType::OneOf(_) => false,
            ProtobufType::Complex(_) => false,
            ProtobufType::Repeated(_) => false,
        }
    }
}

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
            ProtobufType::OneOf(_) => "oneof",
            ProtobufType::Complex(name) => return name.clone(),
            ProtobufType::Repeated(name) => return format!("repeated {}", name.to_string()),
        }.into()
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Protobuf {
    Message(Vec<(String, ProtobufType)>),
    Enum(Vec<String>),
}

impl Model<Protobuf> {
    pub fn convert_rust_to_protobuf(rust_model: &Model<Rust>) -> Model<Protobuf> {
        let mut model = Model {
            name: rust_model.name.clone(),
            imports: rust_model.imports.clone(),
            definitions: Vec::with_capacity(rust_model.definitions.len()),
        };
        for Definition(name, rust) in rust_model.definitions.iter() {
            let proto = Self::definition_to_protobuf(rust);
            model
                .definitions
                .push(Definition(proto_definition_name(name), proto));
        }
        model
    }

    pub fn definition_to_protobuf(rust: &Rust) -> Protobuf {
        match rust {
            Rust::Struct(fields) => {
                let mut proto_fields = Vec::with_capacity(fields.len());
                for (name, rust) in fields.iter() {
                    proto_fields.push((
                        proto_field_name(name),
                        Self::definition_type_to_protobuf_type(rust),
                    ));
                }

                Protobuf::Message(proto_fields)
            }
            Rust::Enum(variants) => {
                Protobuf::Enum(variants.iter().map(|v| proto_variant_name(v)).collect())
            }
            Rust::DataEnum(variants) => {
                let mut proto_variants = Vec::with_capacity(variants.len());
                for (name, rust) in variants.iter() {
                    proto_variants.push((
                        proto_field_name(name),
                        Self::definition_type_to_protobuf_type(rust),
                    ))
                }
                Protobuf::Message(vec![(
                    DATAENUM_VARIABLE_NAME_REPLACEMENT.into(),
                    ProtobufType::OneOf(proto_variants),
                )])
            }
            Rust::TupleStruct(inner) => Protobuf::Message(vec![(
                TUPLE_VARIABLE_NAME_REPLACEMENT.into(),
                Self::definition_type_to_protobuf_type(inner),
            )]),
        }
    }

    pub fn definition_type_to_protobuf_type(rust_type: &RustType) -> ProtobufType {
        match rust_type {
            RustType::Bool => ProtobufType::Bool,
            RustType::U8(_) => ProtobufType::UInt32,
            RustType::I8(_) => ProtobufType::SInt32,
            RustType::U16(_) => ProtobufType::UInt32,
            RustType::I16(_) => ProtobufType::SInt32,
            RustType::U32(_) => ProtobufType::UInt32,
            RustType::I32(_) => ProtobufType::SInt32,
            RustType::U64(_) => ProtobufType::UInt64,
            RustType::I64(_) => ProtobufType::SInt64,
            RustType::String => ProtobufType::String,
            RustType::VecU8 => ProtobufType::Bytes,

            RustType::Complex(complex) => ProtobufType::Complex(complex.clone()),

            RustType::Option(inner) => {
                // in protobuf everything is optional...
                Self::definition_type_to_protobuf_type(inner)
            }

            RustType::Vec(inner) => {
                ProtobufType::Repeated(Box::new(Self::definition_type_to_protobuf_type(inner)))
            }
        }
    }
}

pub fn proto_field_name(name: &str) -> String {
    rust_module_name(name)
}

pub fn proto_variant_name(name: &str) -> String {
    rust_variant_name(name)
}

pub fn proto_definition_name(name: &str) -> String {
    rust_struct_or_enum_name(name)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_non_definitions_rust_to_protobuf() {
        let mut model_rust = Model::default();
        model_rust.name = "ModelWithOriginOfRust".into();
        model_rust.imports = vec![Import {
            what: vec!["a".into(), "b".into()],
            from: "some_very_specific_module".into(),
        }];
        let model_proto = model_rust.to_protobuf();
        assert_eq!(model_rust.name, model_proto.name);
        assert_eq!(model_rust.imports, model_proto.imports);
        assert!(model_proto.definitions.is_empty());
    }

    #[test]
    fn test_simple_rust_struct_to_protobuf() {
        test_model_definition_conversion(
            vec![Definition(
                "Mine".into(),
                Rust::Struct(vec![("field".into(), RustType::U8(Range(0, 255)))]),
            )],
            vec![Definition(
                "Mine".into(),
                Protobuf::Message(vec![("field".into(), ProtobufType::UInt32)]),
            )],
        );
    }

    #[test]
    fn test_simple_rust_tuple_to_protobuf() {
        test_model_definition_conversion(
            vec![Definition(
                "SuchTuple".into(),
                Rust::TupleStruct(RustType::Complex("VeryWow".into())),
            )],
            vec![Definition(
                "SuchTuple".into(),
                Protobuf::Message(vec![(
                    TUPLE_VARIABLE_NAME_REPLACEMENT.into(),
                    ProtobufType::Complex("VeryWow".into()),
                )]),
            )],
        );
    }

    #[test]
    fn test_simple_rust_enum_to_protobuf() {
        test_model_definition_conversion(
            vec![Definition(
                "SuchEnum".into(),
                Rust::Enum(vec!["VeryWow".into(), "MuchGreat".into()]),
            )],
            vec![Definition(
                "SuchEnum".into(),
                Protobuf::Enum(vec!["VeryWow".into(), "MuchGreat".into()]),
            )],
        );
    }

    #[test]
    fn test_simple_rust_strucht_with_option_to_protobuf() {
        test_model_definition_conversion(
            vec![Definition(
                "SuchStruct".into(),
                Rust::Struct(vec![(
                    "very_optional".into(),
                    RustType::Option(Box::new(RustType::String)),
                )]),
            )],
            vec![Definition(
                "SuchStruct".into(),
                Protobuf::Message(vec![("very_optional".into(), ProtobufType::String)]),
            )],
        );
    }

    #[test]
    fn test_simple_rust_data_enum_to_protobuf() {
        test_model_definition_conversion(
            vec![Definition(
                "SuchDataEnum".into(),
                Rust::DataEnum(vec![("MuchVariant".into(), RustType::String)]),
            )],
            vec![Definition(
                "SuchDataEnum".into(),
                Protobuf::Message(vec![(
                    DATAENUM_VARIABLE_NAME_REPLACEMENT.into(),
                    ProtobufType::OneOf(vec![("much_variant".into(), ProtobufType::String)]),
                )]),
            )],
        );
    }

    #[test]
    fn test_multiple_rust_defs_to_protobuf() {
        test_model_definition_conversion(
            vec![
                Definition("First".into(), Rust::Enum(vec!["A".into(), "B".into()])),
                Definition("Second".into(), Rust::TupleStruct(RustType::VecU8)),
            ],
            vec![
                Definition("First".into(), Protobuf::Enum(vec!["A".into(), "B".into()])),
                Definition(
                    "Second".into(),
                    Protobuf::Message(vec![(
                        TUPLE_VARIABLE_NAME_REPLACEMENT.into(),
                        ProtobufType::Bytes,
                    )]),
                ),
            ],
        )
    }

    fn test_model_definition_conversion(
        rust: Vec<Definition<Rust>>,
        proto: Vec<Definition<Protobuf>>,
    ) {
        let mut model_rust = Model::default();
        model_rust.definitions = rust;
        let model_proto = model_rust.to_protobuf();
        assert_eq!(proto.len(), model_proto.definitions.len());
        assert_eq!(proto, model_proto.definitions)
    }
}
