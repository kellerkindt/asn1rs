use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;

use model::Definition;
use model::Field;
use model::ProtobufType;
use model::Role;

use gen::rust::GeneratorSupplement;
use gen::rust::RustCodeGenerator;

use io::protobuf::Format as ProtobufFormat;

pub struct ProtobufGenerator;
impl GeneratorSupplement for ProtobufGenerator {
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

    fn impl_supplement(&self, scope: &mut Scope, impl_for: &str, definition: &Definition) {
        Self::impl_eq_fn(
            Self::new_eq_fn(Self::new_eq_impl(scope, impl_for)),
            definition,
        );

        let serializable_impl = Self::new_protobuf_serializable_impl(scope, impl_for);

        Self::impl_format_fn(Self::new_format_fn(serializable_impl), definition);
        Self::impl_read_fn(Self::new_read_fn(serializable_impl), definition);
        Self::impl_write_fn(Self::new_write_fn(serializable_impl), definition);
    }
}

impl ProtobufGenerator {
    const CODEC: &'static str = "Protobuf";

    fn new_protobuf_serializable_impl<'a>(scope: &'a mut Scope, impl_for: &str) -> &'a mut Impl {
        RustCodeGenerator::new_serializable_impl(scope, impl_for, Self::CODEC)
    }

    fn new_read_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        RustCodeGenerator::new_read_fn(implementation, Self::CODEC)
    }

    fn impl_read_fn(function: &mut Function, definition: &Definition) {
        match definition {
            Definition::SequenceOf(name, aliased) => {
                Self::impl_read_fn_for_sequence_of(function, name, aliased);
            }
            Definition::Sequence(name, fields) => {
                Self::impl_read_fn_for_sequence(function, name, &fields[..]);
            }
            Definition::Enumerated(name, variants) => {
                Self::impl_read_fn_for_enumeration(function, name, &variants[..]);
            }
        };
    }

    fn impl_read_fn_for_sequence_of(function: &mut Function, _name: &str, aliased: &Role) {
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
        match aliased {
            Role::Custom(custom) => block_reader.line(format!(
                "me.values.push({}::read_protobuf(reader)?);",
                custom
            )),
            r => block_reader.line(format!(
                "me.values.push(reader.read_{}()?{});",
                r.clone().into_protobuf().to_string(),
                Self::get_as_rust_type_statement(r),
            )),
        };
        block_while.push_block(block_reader);
        function.push_block(block_while);
        function.line("Ok(me)");
    }

    fn impl_read_fn_for_sequence(function: &mut Function, name: &str, fields: &[Field]) {
        for field in fields.iter() {
            function.line(format!(
                "let mut read_{} = None;",
                RustCodeGenerator::rust_field_name(&field.name, false)
            ));
        }

        let mut block_reader_loop = Block::new("while let Ok(tag) = reader.read_tag()");
        let mut block_match_tag = Block::new("match tag.0");
        block_match_tag.line("0 => break,");

        for (prev_tag, field) in fields.iter().enumerate() {
            match &field.role {
                Role::Custom(name) => {
                    let mut block_case = Block::new(&format!(
                        "{} => read_{} = Some(",
                        prev_tag + 1,
                        RustCodeGenerator::rust_field_name(&field.name, false)
                    ));
                    let mut block_case_if = Block::new(&format!(
                        "if {}::{}_format() == {}Format::LengthDelimited",
                        name,
                        Self::CODEC.to_lowercase(),
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
                    block_match_tag.line(format!(
                        "{} => read_{} = Some({}),",
                        prev_tag + 1,
                        RustCodeGenerator::rust_field_name(&field.name, false),
                        format!(
                            "reader.read_{}()?",
                            role.clone().into_protobuf().to_string(),
                        )
                    ));
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
        for field in fields.iter() {
            return_block.line(&format!(
                "{}: read_{}.map(|v| v{}){},",
                RustCodeGenerator::rust_field_name(&field.name, true),
                RustCodeGenerator::rust_field_name(&field.name, false),
                Self::get_as_rust_type_statement(&field.role),
                if field.optional {
                    "".into()
                } else {
                    format!(
                        ".unwrap_or({}::default())",
                        field.role.clone().into_rust().to_string()
                    )
                },
            ));
        }

        return_block.after(")");
        function.push_block(return_block);
    }

    fn impl_read_fn_for_enumeration(function: &mut Function, name: &str, variants: &[String]) {
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

    fn new_write_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        RustCodeGenerator::new_write_fn(implementation, Self::CODEC)
    }

    fn impl_write_fn(function: &mut Function, definition: &Definition) {
        match definition {
            Definition::SequenceOf(name, aliased) => {
                Self::impl_write_fn_for_sequence_of(function, name, aliased);
            }
            Definition::Sequence(name, fields) => {
                Self::impl_write_fn_for_sequence(function, name, &fields[..]);
            }
            Definition::Enumerated(name, variants) => {
                Self::impl_write_fn_for_enumeration(function, name, &variants[..]);
            }
        };
        function.line("Ok(())");
    }

    fn impl_write_fn_for_sequence_of(function: &mut Function, _name: &str, aliased: &Role) {
        let mut block_writer = Block::new("");
        let mut block_for = Block::new("for value in self.values.iter()");
        block_for.line(format!(
            "writer.write_tag(1, {})?;",
            Self::role_to_format(aliased),
        ));
        block_for.line("let mut bytes = Vec::new();");
        match aliased {
            Role::Custom(_custom) => {
                block_for.line(format!(
                    "value.write_protobuf(&mut bytes as &mut {}Writer)?;",
                    Self::CODEC
                ));
            }
            r => {
                block_for.line(format!(
                    "(&mut bytes as &mut {}Writer).write_{}(*value{})?;",
                    Self::CODEC,
                    r.clone().into_protobuf().to_string(),
                    Self::get_as_protobuf_type_statement(r),
                ));
            }
        };
        block_for.line("writer.write_bytes(&bytes[..])?;");
        block_writer.push_block(block_for);
        function.push_block(block_writer);
    }

    fn impl_write_fn_for_sequence(function: &mut Function, _name: &str, fields: &[Field]) {
        for (prev_tag, field) in fields.iter().enumerate() {
            let block_: &mut Function = function;
            let mut block = if field.optional {
                Block::new(&format!(
                    "if let Some(ref {}) = self.{}",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ))
            } else {
                Block::new("")
            };

            match &field.role {
                Role::Custom(_custom) => {
                    let format_line =
                        format!("{}::{}_format()", _custom, Self::CODEC.to_lowercase());
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
                        if field.optional { "" } else { "self." },
                        RustCodeGenerator::rust_field_name(&field.name, true),
                        Self::CODEC,
                    ));
                    block_if.line("writer.write_bytes(&vec[..])?;");

                    let mut block_el = Block::new("else");
                    block_el.line(format!(
                        "{}{}.write_protobuf(writer)?;",
                        if field.optional { "" } else { "self." },
                        RustCodeGenerator::rust_field_name(&field.name, true),
                    ));

                    block.push_block(block_if);
                    block.push_block(block_el);
                }
                r => {
                    block.line(format!(
                        "writer.write_tagged_{}({}, {}{}{})?;",
                        r.clone().into_protobuf().to_string(),
                        prev_tag + 1,
                        if ProtobufType::String == r.clone().into_protobuf() {
                            if field.optional {
                                ""
                            } else {
                                "&self."
                            }
                        } else {
                            if field.optional {
                                "*"
                            } else {
                                "self."
                            }
                        },
                        RustCodeGenerator::rust_field_name(&field.name, true),
                        Self::get_as_protobuf_type_statement(r),
                    ));
                }
            };
            block_.push_block(block);
        }
    }

    fn impl_write_fn_for_enumeration(function: &mut Function, name: &str, variants: &[String]) {
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

    fn new_format_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        implementation
            .new_fn(&format!("{}_format", Self::CODEC.to_lowercase()))
            .ret(format!("{}Format", Self::CODEC))
    }

    fn impl_format_fn(function: &mut Function, definition: &Definition) {
        let format = match definition {
            Definition::SequenceOf(_, _) => ProtobufFormat::LengthDelimited,
            Definition::Sequence(_, _) => ProtobufFormat::LengthDelimited,
            Definition::Enumerated(_, _) => ProtobufFormat::VarInt,
        };
        function.line(format!("{}Format::{}", Self::CODEC, format.to_string()));
    }

    fn new_eq_impl<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Impl {
        scope
            .new_impl(name)
            .impl_trait(&format!("{}Eq", Self::CODEC))
    }

    fn new_eq_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        implementation
            .new_fn(&format!("{}_eq", Self::CODEC.to_lowercase()))
            .ret("bool")
            .arg_ref_self()
            .arg("other", format!("&Self"))
    }

    fn impl_eq_fn(function: &mut Function, definition: &Definition) {
        match definition {
            Definition::SequenceOf(_, _) => {
                function.line(format!(
                    "self.values.{}_eq(&other.values)",
                    Self::CODEC.to_lowercase()
                ));
            }
            Definition::Sequence(_, fields) => {
                for (num, field) in fields.iter().enumerate() {
                    if num > 0 {
                        function.line("&&");
                    }
                    let field_name = RustCodeGenerator::rust_field_name(&field.name, true);
                    function.line(&format!(
                        "self.{}.{}_eq(&other.{})",
                        field_name,
                        Self::CODEC.to_lowercase(),
                        field_name
                    ));
                }
            }
            Definition::Enumerated(_, _) => {
                function.line("self == other");
            }
        }
    }

    fn role_to_format(role: &Role) -> String {
        match role.clone().into_protobuf() {
            ProtobufType::Bool => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::SFixed32 => format!("{}Format::Fixed32", Self::CODEC),
            ProtobufType::SFixed64 => format!("{}Format::Fixed64", Self::CODEC),
            ProtobufType::UInt32 => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::UInt64 => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::SInt32 => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::SInt64 => format!("{}Format::VarInt", Self::CODEC),
            ProtobufType::String => format!("{}Format::LengthDelimited", Self::CODEC),
            ProtobufType::Complex(complex) => {
                format!("{}::{}_format()", complex, Self::CODEC.to_lowercase())
            }
        }
    }

    fn get_as_protobuf_type_statement(role: &Role) -> String {
        let role_rust = role.clone().into_rust();
        let proto_rust = role.clone().into_protobuf().into_rust();

        if role_rust != proto_rust {
            format!(" as {}", proto_rust.to_string())
        } else {
            "".into()
        }
    }

    fn get_as_rust_type_statement(role: &Role) -> String {
        let role_rust = role.clone().into_rust();
        let proto_rust = role.clone().into_protobuf().into_rust();

        if role_rust != proto_rust {
            format!(" as {}", role_rust.to_string())
        } else {
            "".into()
        }
    }
}
