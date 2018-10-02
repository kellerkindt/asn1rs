use model::rust::*;
use model::*;

use backtrace::Backtrace;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Protobuf {
    Bool,
    SFixed32,
    SFixed64,
    UInt32,
    UInt64,
    SInt32,
    SInt64,
    String,
    Bytes,

    Repeated(Box<Protobuf>),
    Message(Vec<(String, Protobuf)>),
    Enum(Vec<String>),
    /// Indicates a complex, custom type that is
    /// not one of rusts known types
    Complex(String),
}

impl Protobuf {
    pub fn is_primitive(&self) -> bool {
        if let Protobuf::Complex(_) = self {
            false
        } else {
            true
        }
    }
}

/*
impl ToString for Protobuf {
    fn to_string(&self) -> String {
        match self {
            Protobuf::Bool => "bool",
            Protobuf::SFixed32 => "sfixed32",
            Protobuf::SFixed64 => "sfixed64",
            Protobuf::UInt32 => "uint32",
            Protobuf::UInt64 => "uint64",
            Protobuf::SInt32 => "sint32",
            Protobuf::SInt64 => "sint64",
            Protobuf::String => "string",
            Protobuf::Bytes => "bytes",
            Protobuf::Complex(name) => return name.clone(),
        }.into()
    }
}*/

impl Model<Protobuf> {
    pub fn convert_rust_to_protobuf(rust_model: &Model<Rust>) -> Model<Protobuf> {
        let mut model = Model {
            name: rust_model.name.clone(),
            imports: rust_model.imports.clone(),
            definitions: Vec::with_capacity(rust_model.definitions.len()),
        };
        for Definition(name, rust) in rust_model.definitions.iter() {
            //Self::definition_to_protobuf(&name, rust, &mut model.definitions);
        }
        model
    }/*

    pub fn definition_to_protobuf(
        name: &str,
        rust: &Rust,
        defs: &mut Vec<Definition<Protobuf>>,
    ) -> Protobuf {
        Ok(match rust {
            Rust::Bool => Protobuf::Bool,
            Rust::U8(_) => Protobuf::UInt32,
            Rust::I8(_) => Protobuf::SInt32,
            Rust::U16(_) => Protobuf::UInt32,
            Rust::I16(_) => Protobuf::SInt32,
            Rust::U32(_) => Protobuf::UInt32,
            Rust::I32(_) => Protobuf::SInt32,
            Rust::U64(_) => Protobuf::UInt64,
            Rust::I64(_) => Protobuf::SInt64,
            Rust::String => Protobuf::String,
            Rust::VecU8 => Protobuf::Bytes,
            Rust::Vec(inner) => {
                Protobuf::Repeated(Box::new(Self::definition_to_protobuf(name, inner, defs)?))
            }
            Rust::Option(inner) => {
                // in protobuf everything is optional...
                Self::definition_to_protobuf(name, inner, defs)
            }
            Rust::Struct(fields) => {
                let mut proto_fields = Vec::with_capacity(fields.len());
                for (name, rust) in fields.iter() {
                    match rust {
                        Rust::Struct(_) | Rust::Enum(_) | Rust::DataEnum(_) | Rust::TupleStruct(_) => {
                            return Err(Error::IllegalModel(format!("Illegal inner anonymous type in rust model: {:?}", rust), Backtrace::new()));
                        }
                        _ => {
                            proto_fields.push((name.clone(), Self::definition_to_protobuf(name, rust, defs)?));
                        }
                    }
                }
                Protobuf::Message(proto_fields)
            }
            Rust::Enum(variants) => Protobuf::Enum(variants.clone()),
            Rust::DataEnum(_) => {}
            Rust::TupleStruct(_) => {}
            Rust::Complex(_) => {}
        })
    }*/
}

#[cfg(test)]
mod test {
    use super::*;
    use model::test::*;
    use parser::Parser;
}
