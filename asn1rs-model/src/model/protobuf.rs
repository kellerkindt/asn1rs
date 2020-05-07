use crate::model::rust::*;
use crate::model::*;

const TUPLE_VARIABLE_NAME_REPLACEMENT: &str = "value";
const DATAENUM_VARIABLE_NAME_REPLACEMENT: &str = "value";

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum ProtobufType {
    Bool,
    #[allow(dead_code)]
    SFixed32,
    #[allow(dead_code)]
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
    pub fn from(rust: &RustType) -> ProtobufType {
        Model::definition_type_to_protobuf_type(rust)
    }

    pub fn to_rust(&self) -> RustType {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
        match self {
            ProtobufType::Bool => RustType::Bool,
            ProtobufType::SFixed32 => RustType::I32(Range(0, i32::max_value())),
            ProtobufType::SFixed64 => RustType::I64(Range(0, i64::max_value())),
            ProtobufType::UInt32 => RustType::U32(Range(0, u32::max_value())),
            ProtobufType::UInt64 => RustType::U64(None),
            ProtobufType::SInt32 => RustType::I32(Range(0, i32::max_value())),
            ProtobufType::SInt64 => RustType::I64(Range(0, i64::max_value())),
            ProtobufType::String => RustType::String,
            ProtobufType::Bytes => RustType::VecU8,
            ProtobufType::Repeated(inner) => RustType::Vec(Box::new(inner.to_rust())),
            ProtobufType::OneOf(_) => panic!("ProtobufType::OneOf cannot be mapped to a RustType"),
            ProtobufType::Complex(name) => RustType::Complex(name.clone()),
        }
    }

    pub fn is_primitive(&self) -> bool {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
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
        }
        .into()
    }
}

pub trait ToProtobufType {
    fn to_protobuf(&self) -> ProtobufType;
}

impl ToProtobufType for RustType {
    fn to_protobuf(&self) -> ProtobufType {
        ProtobufType::from(self)
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
        for Definition(name, rust) in &rust_model.definitions {
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
                for field in fields.iter() {
                    proto_fields.push((
                        proto_field_name(field.name()),
                        Self::definition_type_to_protobuf_type(field.r#type()),
                    ));
                }

                Protobuf::Message(proto_fields)
            }
            Rust::Enum(r_enum) => {
                Protobuf::Enum(r_enum.variants().map(|v| proto_variant_name(v)).collect())
            }
            Rust::DataEnum(enumeration) => {
                let mut proto_enum = Vec::with_capacity(enumeration.len());
                for variant in enumeration.variants() {
                    proto_enum.push((
                        proto_field_name(variant.name()),
                        Self::definition_type_to_protobuf_type(variant.r#type()),
                    ))
                }
                Protobuf::Message(vec![(
                    DATAENUM_VARIABLE_NAME_REPLACEMENT.into(),
                    ProtobufType::OneOf(proto_enum),
                )])
            }
            Rust::TupleStruct(inner) => Protobuf::Message(vec![(
                TUPLE_VARIABLE_NAME_REPLACEMENT.into(),
                Self::definition_type_to_protobuf_type(inner),
            )]),
        }
    }

    pub fn definition_type_to_protobuf_type(rust_type: &RustType) -> ProtobufType {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
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

pub trait ToProtobufModel {
    fn to_protobuf(&self) -> Model<Protobuf>;
}

impl ToProtobufModel for Model<Rust> {
    fn to_protobuf(&self) -> Model<Protobuf> {
        Model::convert_rust_to_protobuf(self)
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
mod tests {
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
            &[Definition(
                "Mine".into(),
                Rust::Struct(vec![Field::from_name_type(
                    "field",
                    RustType::U8(Range(0, 255)),
                )]),
            )],
            &[Definition(
                "Mine".into(),
                Protobuf::Message(vec![("field".into(), ProtobufType::UInt32)]),
            )],
        );
    }

    #[test]
    fn test_simple_rust_tuple_to_protobuf() {
        test_model_definition_conversion(
            &[Definition(
                "SuchTuple".into(),
                Rust::TupleStruct(RustType::Complex("VeryWow".into())),
            )],
            &[Definition(
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
            &[Definition(
                "SuchEnum".into(),
                Rust::Enum(vec!["VeryWow".into(), "MuchGreat".into()].into()),
            )],
            &[Definition(
                "SuchEnum".into(),
                Protobuf::Enum(vec!["VeryWow".into(), "MuchGreat".into()]),
            )],
        );
    }

    #[test]
    fn test_simple_rust_strucht_with_option_to_protobuf() {
        test_model_definition_conversion(
            &[Definition(
                "SuchStruct".into(),
                Rust::Struct(vec![Field::from_name_type(
                    "very_optional",
                    RustType::Option(Box::new(RustType::String)),
                )]),
            )],
            &[Definition(
                "SuchStruct".into(),
                Protobuf::Message(vec![("very_optional".into(), ProtobufType::String)]),
            )],
        );
    }

    #[test]
    fn test_simple_rust_data_enum_to_protobuf() {
        test_model_definition_conversion(
            &[Definition(
                "SuchDataEnum".into(),
                Rust::DataEnum(
                    vec![DataVariant::from_name_type("MuchVariant", RustType::String)].into(),
                ),
            )],
            &[Definition(
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
            &[
                Definition(
                    "First".into(),
                    Rust::Enum(vec!["A".into(), "B".into()].into()),
                ),
                Definition("Second".into(), Rust::TupleStruct(RustType::VecU8)),
            ],
            &[
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

    fn test_model_definition_conversion(rust: &[Definition<Rust>], proto: &[Definition<Protobuf>]) {
        let mut model_rust = Model::default();
        model_rust.definitions = rust.to_vec();
        let model_proto = model_rust.to_protobuf();
        assert_eq!(proto.len(), model_proto.definitions.len());
        assert_eq!(proto, &model_proto.definitions[..])
    }
}
