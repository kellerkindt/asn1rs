use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;

use model::Definition;
use model::Field;
use model::Range;
use model::Rust;
use model::RustType;

use gen::rust::GeneratorSupplement;
use gen::rust::RustCodeGenerator;

pub struct UperSerializer;
impl GeneratorSupplement<Rust> for UperSerializer {
    fn add_imports(&self, scope: &mut Scope) {
        scope.import("asn1c::io::uper", Self::CODEC);
        scope.import("asn1c::io::uper", &format!("Error as {}Error", Self::CODEC));
        scope.import(
            "asn1c::io::uper",
            &format!("Reader as {}Reader", Self::CODEC),
        );
        scope.import(
            "asn1c::io::uper",
            &format!("Writer as {}Writer", Self::CODEC),
        );
    }

    fn impl_supplement(&self, scope: &mut Scope, definition: &Definition<Rust>) {
        let serializable_implementation = Self::new_uper_serializable_impl(scope, &definition.0);
        Self::impl_read_fn(Self::new_read_fn(serializable_implementation), definition);
        Self::impl_write_fn(Self::new_write_fn(serializable_implementation), definition);
    }
}

impl UperSerializer {
    const CODEC: &'static str = "Uper";

    fn new_uper_serializable_impl<'a>(scope: &'a mut Scope, impl_for: &str) -> &'a mut Impl {
        RustCodeGenerator::new_serializable_impl(scope, impl_for, Self::CODEC)
    }

    fn new_read_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        RustCodeGenerator::new_read_fn(implementation, Self::CODEC)
    }

    fn impl_read_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::TupleStruct(aliased) => {
                Self::impl_read_fn_for_tuple_struct(function, name, aliased);
            }
            Rust::Struct(fields) => {
                for (field_name, field_type) in fields.iter() {
                    Self::impl_read_fn_header_for_type(function, field_name, field_type);
                }
                Self::impl_read_fn_for_struct(function, name, fields);
            }
            Rust::Enum(variants) => {
                Self::impl_read_fn_for_enum(function, name, &variants[..]);
            }
            Rust::DataEnum(variants) => {
                Self::impl_read_fn_for_data_enum(function, name, &variants[..]);
            }
        };
    }

    fn impl_read_fn_for_tuple_struct(function: &mut Function, name: &str, aliased: &RustType) {
        Self::impl_read_fn_header_for_type(function, name, aliased);
        function.push_block({
            let mut block = Block::new(&format!("Ok({}(", name));
            Self::impl_read_fn_for_type(&mut block, &aliased.to_inner_type_string(), None, aliased);
            block.after("))");
            block
        });
    }

    fn impl_read_fn_header_for_type(function: &mut Function, name: &str, aliased: &RustType) {
        if let RustType::Option(_) = aliased {
            function.line(&format!("let {} = reader.read_bit()?;", name));
        }
    }

    fn impl_read_fn_for_type(
        block: &mut Block,
        type_name: &str,
        field_name: Option<&str>,
        rust: &RustType,
    ) {
        match rust {
            RustType::Bool => {
                block.line("reader.read_bit()?");
            }
            RustType::U8(_)
            | RustType::I8(_)
            | RustType::U16(_)
            | RustType::I16(_)
            | RustType::U32(_)
            | RustType::I32(_)
            | RustType::U64(Some(_))
            | RustType::I64(_) => {
                block.line(format!(
                    "reader.read_int((Self::{}min() as i64, Self::{}max() as i64))?.into()",
                    if let Some(field_name) = field_name {
                        format!("{}_", field_name)
                    } else {
                        String::default()
                    },
                    if let Some(field_name) = field_name {
                        format!("{}_", field_name)
                    } else {
                        String::default()
                    },
                ));
            }
            RustType::U64(None) => {
                block.line("reader.read_int_max()?");
            }
            RustType::String => {
                block.line("reader.read_utf8_string()?");
            }
            RustType::VecU8 => {
                block.line("reader.read_octet_string(None)?");
            }
            RustType::Vec(inner) => {
                block.line("let len = reader.read_length_determinant()?;");
                block.line("let mut values = Vec::with_capacity(len);");
                let mut for_block = Block::new("for _ in 0..len");
                for_block.push_block({
                    let mut inner_block = Block::new("values.push(");
                    Self::impl_read_fn_for_type(
                        &mut inner_block,
                        &inner.to_inner_type_string(),
                        None,
                        inner,
                    );
                    inner_block.after(");");
                    inner_block
                });
                block.push_block(for_block);
                block.line("values");
            }
            RustType::Option(inner) => {
                let mut if_block = Block::new(&format!("if {}", field_name.unwrap_or("value")));
                let mut if_true_block = Block::new("Some(");
                Self::impl_read_fn_for_type(
                    &mut if_true_block,
                    &inner.to_inner_type_string(),
                    field_name,
                    inner,
                );
                if_true_block.after(")");
                if_block.push_block(if_true_block);
                let mut else_block = Block::new("else");
                else_block.line("None");
                block.push_block(if_block);
                block.push_block(else_block);
            }
            RustType::Complex(inner) => {
                block.line(format!("{}::read_uper(reader)?", type_name));
            }
        };
    }

    fn impl_read_fn_for_struct(function: &mut Function, name: &str, fields: &[(String, RustType)]) {
        function.line("let mut me = Self::default();");
        for (field_name, field_type) in fields.iter() {
            let mut block = Block::new(&format!("me.{} = ", field_name));
            Self::impl_read_fn_for_type(
                &mut block,
                &field_type.to_inner_type_string(),
                Some(field_name),
                field_type,
            );
            block.after(";");
            function.push_block(block);
        }
        function.line("Ok(me)");
    }

    fn impl_read_fn_for_enum(function: &mut Function, name: &str, variants: &[String]) {
        function.line(format!(
            "let id = reader.read_int((0, {}))?;",
            variants.len() - 1
        ));
        let mut block_match = Block::new("match id");
        for (i, variant) in variants.iter().enumerate() {
            block_match.line(format!("{} => Ok({}::{}),", i, name, variant));
        }
        block_match.line(format!(
            "_ => Err(UperError::ValueNotInRange(id, 0, {}))",
            variants.len() - 1
        ));
        function.push_block(block_match);
    }

    fn impl_read_fn_for_data_enum(
        function: &mut Function,
        name: &str,
        variants: &[(String, RustType)],
    ) {
        if variants.len() > 1 {
            function.line(&format!(
                "let variant = reader.read_int((0, {}))?;",
                variants.len() - 1
            ));
        } else {
            function.line("let variant = 0");
        }
        let mut block = Block::new("match variant");
        for (i, (variant, role)) in variants.iter().enumerate() {
            let mut block_case = Block::new(&format!("{} => Ok({}::{}(", i, name, variant));
            Self::impl_read_fn_for_type(&mut block_case, &role.to_inner_type_string(), None, role);
            block_case.after("))");
            block.push_block(block_case);
        }
        block.line(format!(
            "_ => Err(UperError::ValueNotInRange(variant, 0, {}))",
            variants.len() - 1
        ));
        function.push_block(block);
    }

    fn new_write_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        RustCodeGenerator::new_write_fn(implementation, Self::CODEC)
    }

    fn impl_write_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::TupleStruct(inner) => {
                Self::impl_write_fn_for_tuple_struct(function, name, inner);
            }
            Rust::Struct(fields) => {
                for (field_name, field_type) in fields.iter() {
                    Self::impl_write_fn_header_for_type(function, field_name, field_type);
                }
                Self::impl_write_fn_for_struct(function, name, fields);
            }
            Rust::Enum(variants) => {
                Self::impl_write_fn_for_enum(function, name, &variants[..]);
            }
            Rust::DataEnum(_) => {}
        }
        /*
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
            Definition::Choice(name, variants) => {
                Self::impl_write_fn_for_choice(function, name, &variants[..]);
            }
        };
        function.line("Ok(())");
        */
    }

    fn impl_write_fn_for_tuple_struct(function: &mut Function, name: &str, aliased: &RustType) {
        Self::impl_write_fn_header_for_type(function, "self.0", aliased);
        function.push_block({
            let mut block = Block::new("");
            Self::impl_write_fn_for_type(
                &mut block,
                &aliased.to_inner_type_string(),
                if aliased.is_primitive() {
                    Some("self.0")
                } else {
                    Some("&self.0")
                },
                aliased,
            );
            block
        });
    }

    fn impl_write_fn_header_for_type(function: &mut Function, name: &str, aliased: &RustType) {
        if let RustType::Option(_) = aliased {
            function.line(&format!("writer.write_bit({}.is_some())?;", name));
        }
    }

    fn impl_write_fn_for_type(
        block: &mut Block,
        type_name: &str,
        field_name: Option<&str>,
        rust: &RustType,
    ) {
        match rust {
            RustType::Bool => {
                block.line(format!(
                    "reader.write_bit({})?;",
                    field_name.unwrap_or("value")
                ));
            }
            RustType::U8(_)
            | RustType::I8(_)
            | RustType::U16(_)
            | RustType::I16(_)
            | RustType::U32(_)
            | RustType::I32(_)
            | RustType::U64(Some(_))
            | RustType::I64(_) => {
                block.line(format!(
                    "writer.write_int({} as i64, (Self::{}min() as i64, Self::{}max() as i64))?;",
                    field_name.unwrap_or("value"),
                    if let Some(field_name) = field_name {
                        format!("{}_", field_name)
                    } else {
                        String::default()
                    },
                    if let Some(field_name) = field_name {
                        format!("{}_", field_name)
                    } else {
                        String::default()
                    },
                ));
            }
            RustType::U64(None) => {
                block.line(&format!(
                    "writer.write_int_max({})?;",
                    field_name.unwrap_or("value")
                ));
            }
            RustType::String => {
                block.line(&format!(
                    "writer.write_utf8_string({})?;",
                    field_name.unwrap_or("value")
                ));
            }
            RustType::VecU8 => {
                block.line(format!(
                    "writer.writer_octet_string({}, None)?;",
                    field_name.unwrap_or("value")
                ));
            }
            RustType::Vec(inner) => {
                let mut for_block = Block::new(&format!(
                    "for value in {}.iter()",
                    field_name.unwrap_or("value")
                ));
                Self::impl_write_fn_for_type(
                    &mut for_block,
                    &inner.to_inner_type_string(),
                    if inner.is_primitive() {
                        Some("*value")
                    } else {
                        Some("value")
                    },
                    inner,
                );
                block.push_block(for_block);
            }
            RustType::Option(inner) => {
                let mut if_block = Block::new(&format!(
                    "if let Some(value) = {}",
                    field_name.unwrap_or("value")
                ));
                Self::impl_write_fn_for_type(
                    &mut if_block,
                    &inner.to_inner_type_string(),
                    if inner.is_primitive() {
                        Some("*value")
                    } else {
                        Some("value")
                    },
                    inner,
                );
                block.push_block(if_block);
            }
            RustType::Complex(inner) => {
                block.line(format!(
                    "{}.write_uper(writer)?;",
                    field_name.unwrap_or("value")
                ));
            }
        }
    }
    fn impl_write_fn_for_struct(
        function: &mut Function,
        name: &str,
        fields: &[(String, RustType)],
    ) {
        let mut block = Block::new("");
        for (field_name, field_type) in fields.iter() {
            Self::impl_write_fn_for_type(
                &mut block,
                &field_type.to_inner_type_string(),
                Some(&format!("self.{}", field_name)),
                field_type,
            );
        }
        function.push_block(block);
    }

    fn impl_write_fn_for_enum(function: &mut Function, name: &str, variants: &[String]) {
        let mut block = Block::new("match self");
        for (i, variant) in variants.iter().enumerate() {
            block.line(format!(
                "{}::{} => writer.write_int({}, (0, {}))?,",
                name,
                &variant,
                i,
                variants.len() - 1
            ));
        }
        function.push_block(block);
    }

    /*

    fn impl_write_fn_for_sequence_of(function: &mut Function, _name: &str, aliased: &Asn) {
        function.line("writer.write_length_determinant(self.values.len())?;");
        let mut block_for = Block::new("for value in self.values.iter()");
        match aliased {
            Asn::Boolean => block_for.line("writer.write_bit(value)?;"),
            Asn::Integer(_) => block_for.line(format!(
                "writer.write_int(*value as i64, (Self::value_min() as i64, Self::value_max() as i64))?;"
            )),
            Asn::UnsignedMaxInteger => {
                block_for.line("writer.write_int_max(*value)?;")
            }
            Asn::UTF8String => block_for.line("writer.write_utf8_string(&value)?;"),
            Asn::OctetString => block_for.line("writer.write_octet_string(&value[..], None)?;"),
            Asn::TypeReference(_custom) => block_for.line("value.write_uper(writer)?;"),
        };
        function.push_block(block_for);
    }

    fn impl_write_fn_for_sequence(function: &mut Function, _name: &str, fields: &[Field]) {
        // bitmask for optional fields
        for field in fields.iter() {
            if field.optional {
                function.line(format!(
                    "writer.write_bit(self.{}.is_some())?;",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ));
            }
        }

        for field in fields.iter() {
            let line = Self::write_field(
                &field.name,
                &field.role,
                field.role.clone().into_rust().is_primitive(),
                field.optional,
            );
            if field.optional {
                let mut b = Block::new(&format!(
                    "if let Some(ref {}) = self.{}",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ));
                b.line(line);
                function.push_block(b);
            } else {
                function.line(line);
            }
        }
    }

    fn impl_write_fn_for_choice(function: &mut Function, name: &str, variants: &[(String, Asn)]) {
        let mut block = Block::new("match self");
        for (i, (variant, role)) in variants.iter().enumerate() {
            let mut block_case = Block::new(&format!(
                "{}::{}(value) =>",
                name,
                RustCodeGenerator::rust_variant_name(&variant),
            ));
            if variants.len() > 1 {
                block_case.line(format!(
                    "writer.write_int({}, (0, {}))?;",
                    i,
                    variants.len() - 1
                ));
            }
            block_case.line(&Self::write_field(
                if role.clone().into_rust().is_primitive() {
                    "*value"
                } else {
                    "value"
                },
                role,
                false,
                true,
            ));
            block.push_block(block_case);
        }
        function.push_block(block);
    }

    fn read_field(field_name: &str, role: &Asn, optional: bool) -> String {
        match role {
            Asn::Boolean => format!(
                "{}reader.read_bit()?{};",
                if optional { "Some(" } else { "" },
                if optional { ")" } else { "" },
            ),
            Asn::Integer(_) => format!(
                "{}reader.read_int((Self::{}_min() as i64, Self::{}_max() as i64))? as {}{};",
                if optional { "Some(" } else { "" },
                RustCodeGenerator::rust_field_name(&field_name, false),
                RustCodeGenerator::rust_field_name(&field_name, false),
                role.clone().into_rust().to_string(),
                if optional { ")" } else { "" },
            ),
            Asn::UnsignedMaxInteger => format!(
                "{}reader.read_int_max()?{};",
                if optional { "Some(" } else { "" },
                if optional { ")" } else { "" },
            ),
            Asn::UTF8String => format!(
                "{}reader.read_utf8_string()?{};",
                if optional { "Some(" } else { "" },
                if optional { ")" } else { "" },
            ),
            Asn::OctetString => format!(
                "{}reader.read_octet_string(None)?{};",
                if optional { "Some(" } else { "" },
                if optional { ")" } else { "" },
            ),
            Asn::TypeReference(ref _type) => format!(
                "{}{}::read_uper(reader)?{};",
                if optional { "Some(" } else { "" },
                role.clone().into_rust().to_string(),
                if optional { ")" } else { "" },
            ),
        }
    }

    fn write_field(field_name: &str, role: &Asn, primitive: bool, no_self_prefix: bool) -> String {
        let prefix = if no_self_prefix {
            if primitive {
                "*"
            } else {
                ""
            }
        } else {
            "self."
        };
        match role {
            Asn::Boolean => format!(
                "writer.write_bit({}{})?;",
                prefix,
                RustCodeGenerator::rust_field_name(field_name, true),
            ),
            Asn::Integer(_) => format!(
                "writer.write_int({}{} as i64, (Self::{}_min() as i64, Self::{}_max() as i64))?;",
                prefix,
                RustCodeGenerator::rust_field_name(field_name, true),
                RustCodeGenerator::rust_field_name(field_name, false),
                RustCodeGenerator::rust_field_name(field_name, false),
            ),
            Asn::UnsignedMaxInteger => format!(
                "writer.write_int_max({}{})?;",
                prefix,
                RustCodeGenerator::rust_field_name(field_name, true),
            ),
            Asn::OctetString => format!(
                "writer.write_octet_string(&{}{}, None)?;",
                prefix,
                RustCodeGenerator::rust_field_name(field_name, true),
            ),
            Asn::UTF8String => format!(
                "writer.write_utf8_string(&{}{})?;",
                prefix,
                RustCodeGenerator::rust_field_name(field_name, true),
            ),
            Asn::TypeReference(ref _type) => format!(
                "{}{}.write_uper(writer)?;",
                prefix,
                RustCodeGenerator::rust_field_name(field_name, true),
            ),
        }
    }*/
}
