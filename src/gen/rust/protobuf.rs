use crate::gen::rust::GeneratorSupplement;
use crate::gen::rust::RustCodeGenerator;
use crate::io::protobuf::Format as ProtobufFormat;
use crate::model::protobuf::ToProtobufType;
use crate::model::Definition;
use crate::model::ProtobufType;
use crate::model::Rust;
use crate::model::RustType;
use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;

pub struct ProtobufSerializer;

impl GeneratorSupplement<Rust> for ProtobufSerializer {
    fn add_imports(&self, scope: &mut Scope) {
        scope.import("asn1c::io::protobuf", Self::CODEC);
        scope.import(
            "asn1c::io::protobuf",
            &format!("ProtobufEq as {}Eq", Self::CODEC),
        );
        scope.import(
            "asn1c::io::protobuf",
            &format!("Reader as {}Reader", Self::CODEC),
        );
        scope.import(
            "asn1c::io::protobuf",
            &format!("Writer as {}Writer", Self::CODEC),
        );
        scope.import(
            "asn1c::io::protobuf",
            &format!("Error as {}Error", Self::CODEC),
        );
        scope.import(
            "asn1c::io::protobuf",
            &format!("Format as {}Format", Self::CODEC),
        );
    }

    fn impl_supplement(&self, scope: &mut Scope, definition: &Definition<Rust>) {
        Self::impl_eq_fn(
            Self::new_eq_fn(Self::new_eq_impl(scope, &definition.0)),
            definition,
        );

        let Definition(name, _) = definition;
        let serializable_impl = Self::new_protobuf_serializable_impl(scope, name);

        Self::impl_format_fn(Self::new_format_fn(serializable_impl), definition);
        Self::impl_read_fn(Self::new_read_fn(serializable_impl), definition);
        Self::impl_write_fn(Self::new_write_fn(serializable_impl), definition);
    }
}

// TODO refactor, see UperSerializer (recursive type serialization), current impl does not support nested Vec<_>s
impl ProtobufSerializer {
    const CODEC: &'static str = "Protobuf";

    fn new_protobuf_serializable_impl<'a>(scope: &'a mut Scope, impl_for: &str) -> &'a mut Impl {
        RustCodeGenerator::new_serializable_impl(scope, impl_for, Self::CODEC)
    }

    fn new_read_fn(implementation: &mut Impl) -> &mut Function {
        RustCodeGenerator::new_read_fn(implementation, Self::CODEC)
    }

    fn impl_read_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::TupleStruct(aliased) => {
                Self::impl_read_fn_for_tuple_struct(function, aliased);
            }
            Rust::Struct(fields) => {
                Self::impl_read_fn_for_struct(function, name, &fields[..]);
            }
            Rust::Enum(variants) => {
                Self::impl_read_fn_for_enum(function, name, &variants[..]);
            }
            Rust::DataEnum(variants) => {
                Self::impl_read_fn_for_data_enum(function, name, &variants[..]);
            }
        };
    }

    fn impl_read_fn_for_tuple_struct(function: &mut Function, aliased: &RustType) {
        function.line("let mut me = Self::default();");

        let mut block_while = Block::new("while let Ok(tag) = reader.read_tag()");
        block_while.line(format!(
            "if tag.0 != 1 {{ return Err({}Error::invalid_tag_received(tag.0)); }}",
            Self::CODEC
        ));
        block_while.line(format!("if tag.1 != {}Format::LengthDelimited {{ return Err({}Error::unexpected_format(tag.1)); }}", Self::CODEC, Self::CODEC));
        block_while.line("let bytes = reader.read_bytes()?;");
        let mut block_reader = Block::new("");
        block_reader.line(format!(
            "let reader = &mut &bytes[..] as &mut {}Reader;",
            Self::CODEC
        ));

        match aliased.clone().into_inner_type() {
            RustType::Complex(custom) => {
                block_reader.line(format!("me.0.push({}::read_protobuf(reader)?);", custom))
            }
            r => block_reader.line(format!(
                "me.0.push(reader.read_{}()?{});",
                r.to_protobuf().to_string(),
                Self::get_as_rust_type_statement(&r),
            )),
        };
        block_while.push_block(block_reader);
        function.push_block(block_while);
        function.line("Ok(me)");
    }

    fn impl_read_fn_for_struct(function: &mut Function, name: &str, fields: &[(String, RustType)]) {
        for (field_name, _field_type) in fields.iter() {
            function.line(format!(
                "let mut read_{} = None;",
                RustCodeGenerator::rust_field_name(&field_name, false),
            ));
        }

        let mut block_reader_loop = Block::new("while let Ok(tag) = reader.read_tag()");
        let mut block_match_tag = Block::new("match tag.0");
        block_match_tag.line("0 => break,");

        for (prev_tag, (field_name, field_type)) in fields.iter().enumerate() {
            match &field_type.clone().into_inner_type() {
                RustType::Complex(name) => {
                    let mut block_case = Block::new(&format!(
                        "{} => read_{}{}(",
                        prev_tag + 1,
                        RustCodeGenerator::rust_field_name(&field_name, false),
                        if let RustType::Vec(_) = field_type.clone().no_option() {
                            ".get_or_insert_with(Vec::default).push"
                        } else {
                            " = Some"
                        }
                    ));
                    let mut block_case_if = Block::new(&format!(
                        "if tag.1 == {}Format::LengthDelimited",
                        Self::CODEC
                    ));
                    block_case_if.line("let bytes = reader.read_bytes()?;");
                    block_case_if.line(format!(
                        "{}::read_protobuf(&mut &bytes[..] as &mut {}Reader)?",
                        name,
                        Self::CODEC
                    ));
                    let mut block_case_el = Block::new("else");
                    block_case_el.line(format!("{}::read_protobuf(reader)?", name));
                    block_case.push_block(block_case_if);
                    block_case.push_block(block_case_el);
                    block_case.after("),");
                    block_match_tag.push_block(block_case);
                }
                role => {
                    if let RustType::Vec(_) = field_type.clone().no_option() {
                        block_match_tag.line(format!(
                            "{} => read_{}.get_or_insert_with(Vec::default).push({}),",
                            prev_tag + 1,
                            RustCodeGenerator::rust_field_name(&field_name, false),
                            format!("reader.read_{}()?", role.to_protobuf().to_string(),)
                        ));
                    } else {
                        block_match_tag.line(format!(
                            "{} => read_{} = Some({}),",
                            prev_tag + 1,
                            RustCodeGenerator::rust_field_name(&field_name, false),
                            format!("reader.read_{}()?", role.to_protobuf().to_string(),)
                        ));
                    }
                }
            }
        }

        block_match_tag.line(format!(
            "_ => return Err({}Error::invalid_tag_received(tag.0)),",
            Self::CODEC
        ));
        block_reader_loop.push_block(block_match_tag);
        function.push_block(block_reader_loop);
        let mut return_block = Block::new(&format!("Ok({}", name));
        for (field_name, field_type) in fields.iter() {
            let as_rust_statement =
                Self::get_as_rust_type_statement(&field_type.clone().into_inner_type());
            return_block.line(&format!(
                "{}: read_{}{}{},",
                RustCodeGenerator::rust_field_name(&field_name, true),
                RustCodeGenerator::rust_field_name(&field_name, false),
                if as_rust_statement.is_empty() {
                    "".into()
                } else if let RustType::Vec(_) = field_type.clone().no_option() {
                    format!(
                        ".map(|v| v.into_iter().map(|v| v{}).collect())",
                        as_rust_statement
                    )
                } else {
                    format!(".map(|v| v{})", as_rust_statement)
                },
                if let RustType::Option(_) = field_type {
                    ""
                } else {
                    ".unwrap_or_default()"
                },
            ));
        }

        return_block.after(")");
        function.push_block(return_block);
    }

    fn impl_read_fn_for_enum(function: &mut Function, name: &str, variants: &[String]) {
        let mut block_match = Block::new("match reader.read_varint()?");
        for (field, variant) in variants.iter().enumerate() {
            block_match.line(format!(
                "{} => Ok({}::{}),",
                field,
                name,
                RustCodeGenerator::rust_variant_name(&variant),
            ));
        }
        block_match.line(format!(
            "v => Err({}Error::invalid_variant(v as u32))",
            Self::CODEC,
        ));
        function.push_block(block_match);
    }

    fn impl_read_fn_for_data_enum(
        function: &mut Function,
        name: &str,
        variants: &[(String, RustType)],
    ) {
        function.line("let tag = reader.read_tag()?;");
        let mut block_match = Block::new("match tag.0");
        for (field, (variant, role)) in variants.iter().enumerate() {
            let mut block_case = Block::new(&format!(
                "{}{} =>",
                field + 1, // + 1 for protobuf offset
                if role.to_protobuf().is_primitive() {
                    "".into()
                } else {
                    format!(" if tag.1 == {}Format::LengthDelimited", Self::CODEC)
                },
            ));
            let complex_name = if let RustType::Complex(name) = role.clone().into_inner_type() {
                Some(name)
            } else {
                None
            };
            if let Some(complex_name) = complex_name {
                block_case.line("let bytes = reader.read_bytes()?;");
                block_case.line(format!(
                    "let value = {}::read_{}(&mut &bytes[..] as &mut {}Reader)?;",
                    complex_name,
                    Self::CODEC.to_lowercase(),
                    Self::CODEC,
                ));
            } else {
                // primitive
                block_case.line(format!(
                    "let value = reader.read_{}()?;",
                    role.to_protobuf().to_string()
                ));
            }
            block_case.line(format!(
                "Ok({}::{}(value))",
                name,
                RustCodeGenerator::rust_variant_name(variant)
            ));
            block_match.push_block(block_case);
        }
        block_match.line(format!(
            "_ => Err({}Error::unexpected_tag(tag))",
            Self::CODEC
        ));
        function.push_block(block_match);
    }

    fn new_write_fn(implementation: &mut Impl) -> &mut Function {
        RustCodeGenerator::new_write_fn(implementation, Self::CODEC)
    }

    fn impl_write_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::TupleStruct(aliased) => {
                Self::impl_write_fn_for_tuple_struct(function, aliased);
            }
            Rust::Struct(fields) => {
                Self::impl_write_fn_for_struct(function, &fields[..]);
            }
            Rust::Enum(variants) => {
                Self::impl_write_fn_for_enum(function, name, &variants[..]);
            }
            Rust::DataEnum(variants) => {
                Self::impl_write_fn_for_data_enum(function, name, &variants[..]);
            }
        };
        function.line("Ok(())");
    }

    fn impl_write_for_vec_attribute(
        block_writer: &mut Block,
        aliased: &RustType,
        attribute_name: &str,
        tag: usize,
    ) {
        let mut block_for = Block::new(&format!(
            "for value in {}",
            if let RustType::Option(_) = aliased {
                attribute_name.to_string()
            } else {
                format!("&self.{}", attribute_name)
            }
        ));
        block_for.line(format!(
            "writer.write_tag({}, {})?;",
            tag,
            Self::role_to_format(aliased, "value"),
        ));
        block_for.line("let mut bytes = Vec::new();");
        match aliased.clone().into_inner_type() {
            RustType::Complex(_) => {
                block_for.line(format!(
                    "value.write_protobuf(&mut bytes as &mut {}Writer)?;",
                    Self::CODEC
                ));
            }
            r => {
                block_for.line(format!(
                    "(&mut bytes as &mut {}Writer).write_{}({})?;",
                    Self::CODEC,
                    r.to_protobuf().to_string(),
                    Self::get_as_protobuf_type_statement("*value".to_string(), &r),
                ));
            }
        };
        block_for.line("writer.write_bytes(&bytes[..])?;");
        block_writer.push_block(block_for);
    }

    fn impl_write_fn_for_tuple_struct(function: &mut Function, aliased: &RustType) {
        let mut block_writer = Block::new("");
        Self::impl_write_for_vec_attribute(&mut block_writer, aliased, "0", 1);
        function.push_block(block_writer);
    }

    fn impl_write_fn_for_struct(function: &mut Function, fields: &[(String, RustType)]) {
        for (prev_tag, (field_name, field_type)) in fields.iter().enumerate() {
            let block_: &mut Function = function;
            let mut block = if let RustType::Option(_) = field_type {
                Block::new(&format!(
                    "if let Some(ref {}) = self.{}",
                    RustCodeGenerator::rust_field_name(&field_name, true),
                    RustCodeGenerator::rust_field_name(&field_name, true),
                ))
            } else {
                Block::new("")
            };

            match &field_type.clone().no_option() {
                RustType::Vec(_) => {
                    Self::impl_write_for_vec_attribute(
                        &mut block,
                        field_type,
                        &RustCodeGenerator::rust_field_name(&field_name, true),
                        prev_tag + 1,
                    );
                }
                RustType::Complex(_) => {
                    let format_line = format!(
                        "{}{}.{}_format()",
                        if let RustType::Option(_) = field_type {
                            ""
                        } else {
                            "self."
                        },
                        RustCodeGenerator::rust_field_name(&field_name, true),
                        Self::CODEC.to_lowercase()
                    );
                    block.line(format!(
                        "writer.write_tag({}, {})?;",
                        prev_tag + 1,
                        format_line,
                    ));
                    let mut block_if = Block::new(&format!(
                        "if {} == {}Format::LengthDelimited",
                        format_line,
                        Self::CODEC
                    ));
                    block_if.line("let mut vec = Vec::new();");
                    block_if.line(format!(
                        "{}{}.write_protobuf(&mut vec as &mut {}Writer)?;",
                        if let RustType::Option(_) = field_type {
                            ""
                        } else {
                            "self."
                        },
                        RustCodeGenerator::rust_field_name(&field_name, true),
                        Self::CODEC,
                    ));
                    block_if.line("writer.write_bytes(&vec[..])?;");

                    let mut block_el = Block::new("else");
                    block_el.line(format!(
                        "{}{}.write_protobuf(writer)?;",
                        if let RustType::Option(_) = field_type {
                            ""
                        } else {
                            "self."
                        },
                        RustCodeGenerator::rust_field_name(&field_name, true),
                    ));

                    block.push_block(block_if);
                    block.push_block(block_el);
                }
                r => {
                    block.line(format!(
                        "writer.write_tagged_{}({}, {})?;",
                        r.to_protobuf().to_string(),
                        prev_tag + 1,
                        Self::get_as_protobuf_type_statement(
                            format!(
                                "{}{}",
                                if ProtobufType::String == r.to_protobuf()
                                    || RustType::VecU8 == r.to_protobuf().to_rust()
                                {
                                    if let RustType::Option(_) = field_type {
                                        ""
                                    } else {
                                        "&self."
                                    }
                                } else if let RustType::Option(_) = field_type {
                                    "*"
                                } else {
                                    "self."
                                },
                                RustCodeGenerator::rust_field_name(&field_name, true),
                            ),
                            r
                        ),
                    ));
                }
            };
            block_.push_block(block);
        }
    }

    fn impl_write_fn_for_enum(function: &mut Function, name: &str, variants: &[String]) {
        let mut outer_block = Block::new("match self");
        for (field, variant) in variants.iter().enumerate() {
            outer_block.line(format!(
                "{}::{} => writer.write_varint({})?,",
                name,
                RustCodeGenerator::rust_variant_name(&variant),
                field,
            ));
        }
        function.push_block(outer_block);
    }

    fn impl_write_fn_for_data_enum(
        function: &mut Function,
        name: &str,
        variants: &[(String, RustType)],
    ) {
        let mut block_match = Block::new("match self");
        for (field, (variant, role)) in variants.iter().enumerate() {
            let mut block_case = Block::new(&format!(
                "{}::{}(value) =>",
                name,
                RustCodeGenerator::rust_variant_name(&variant),
            ));
            if role.to_protobuf().is_primitive() {
                block_case.line(&format!(
                    "writer.write_tag({}, value.{}_format())?;",
                    field + 1, // + 1 for protobuf offset
                    Self::CODEC.to_lowercase()
                ));
                block_case.line(format!(
                    "writer.write_{}({}value)?;",
                    role.to_protobuf().to_string(),
                    if role.to_protobuf().is_primitive() {
                        "*"
                    } else {
                        ""
                    }
                ));
            } else {
                block_case.line("let mut vec = Vec::new();");
                block_case.line(format!(
                    "value.write_{}(&mut vec as &mut {}Writer)?;",
                    Self::CODEC.to_lowercase(),
                    Self::CODEC
                ));
                block_case.line(&format!(
                    "writer.write_tag({}, {}Format::LengthDelimited)?;",
                    field + 1, // + 1 for protobuf offset
                    Self::CODEC
                ));
                block_case.line("writer.write_bytes(&vec[..])?;");
            }
            block_match.push_block(block_case);
        }
        function.push_block(block_match);
    }

    fn new_format_fn(implementation: &mut Impl) -> &mut Function {
        implementation
            .new_fn(&format!("{}_format", Self::CODEC.to_lowercase()))
            .arg_ref_self()
            .ret(format!("{}Format", Self::CODEC))
    }

    fn impl_format_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        let format = match rust {
            Rust::TupleStruct(_) => Some(ProtobufFormat::LengthDelimited),
            Rust::Struct(_) => Some(ProtobufFormat::LengthDelimited),
            Rust::Enum(_) => Some(ProtobufFormat::VarInt),
            Rust::DataEnum(variants) => {
                let mut block_match = Block::new("match self");
                for (variant, role) in variants.iter() {
                    block_match.line(format!(
                        "{}::{}(value) => {}",
                        name,
                        RustCodeGenerator::rust_variant_name(variant),
                        Self::role_to_format(role, "value"),
                    ));
                }
                function.push_block(block_match);
                None
            }
        };
        if let Some(format) = format {
            function.line(format!("{}Format::{}", Self::CODEC, format.to_string()));
        }
    }

    fn new_eq_impl<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Impl {
        scope
            .new_impl(name)
            .impl_trait(&format!("{}Eq", Self::CODEC))
    }

    fn new_eq_fn(implementation: &mut Impl) -> &mut Function {
        implementation
            .new_fn(&format!("{}_eq", Self::CODEC.to_lowercase()))
            .ret("bool")
            .arg_ref_self()
            .arg("other", "&Self".to_string())
    }

    fn impl_eq_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::TupleStruct(_) => {
                function.line(format!(
                    "self.0.{}_eq(&other.0)",
                    Self::CODEC.to_lowercase()
                ));
            }
            Rust::Struct(fields) => {
                for (num, (field_name, _field_type)) in fields.iter().enumerate() {
                    if num > 0 {
                        function.line("&&");
                    }
                    let field_name = RustCodeGenerator::rust_field_name(&field_name, true);
                    function.line(&format!(
                        "self.{}.{}_eq(&other.{})",
                        field_name,
                        Self::CODEC.to_lowercase(),
                        field_name
                    ));
                }
            }
            Rust::Enum(_) => {
                function.line("self == other");
            }
            Rust::DataEnum(variants) => {
                let mut block_match = Block::new("match self");
                for (variant, _role) in variants.iter() {
                    let mut block_case = Block::new(&format!(
                        "{}::{}(value) => ",
                        name,
                        RustCodeGenerator::rust_variant_name(variant),
                    ));
                    let mut block_if = Block::new(&format!(
                        "if let {}::{}(ref other_value) = other",
                        name,
                        RustCodeGenerator::rust_variant_name(variant),
                    ));
                    block_if.line(format!(
                        "value.{}_eq(other_value)",
                        Self::CODEC.to_lowercase()
                    ));
                    let mut block_else = Block::new("else");
                    block_else.line("false");
                    block_case.push_block(block_if);
                    block_case.push_block(block_else);
                    block_match.push_block(block_case);
                }
                function.push_block(block_match);
            }
        }
    }

    fn role_to_format(role: &RustType, complex_name: &str) -> String {
        match role.to_protobuf() {
            ProtobufType::Bool => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::SFixed32 => format!("{}Format::Fixed32", Self::CODEC),
            ProtobufType::SFixed64 => format!("{}Format::Fixed64", Self::CODEC),
            ProtobufType::UInt32 => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::UInt64 => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::SInt32 => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::SInt64 => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::String => format!("{}Format::LengthDelimited", Self::CODEC),
            ProtobufType::Bytes => format!("{}Format::LengthDelimited", Self::CODEC),
            ProtobufType::OneOf(_) => format!("{}Format::LengthDelimited", Self::CODEC),
            ProtobufType::Repeated(_) => format!("{}Format::LengthDelimited", Self::CODEC),
            ProtobufType::Complex(_complex_type) => {
                format!("{}.{}_format(),", complex_name, Self::CODEC.to_lowercase())
            }
        }
    }

    fn get_as_protobuf_type_statement(string: String, role_rust: &RustType) -> String {
        let proto_rust = role_rust.to_protobuf().to_rust();

        if !role_rust.similar(&proto_rust) {
            format!("{}::from({})", proto_rust.to_string(), string)
        } else {
            string
        }
    }

    fn get_as_rust_type_statement(role_rust: &RustType) -> String {
        let proto_rust = role_rust.to_protobuf().to_rust();

        if role_rust.ne(&proto_rust) {
            format!(" as {}", role_rust.to_string())
        } else {
            "".into()
        }
    }
}
