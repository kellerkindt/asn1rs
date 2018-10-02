use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;

use model::Definition;
use model::Field;
use model::Asn;

use gen::rust::GeneratorSupplement;
use gen::rust::RustCodeGenerator;

pub struct UperGenerator;
impl GeneratorSupplement for UperGenerator {
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

    fn impl_supplement(&self, scope: &mut Scope, impl_for: &str, definition: &Definition) {
        let serializable_implementation = Self::new_uper_serializable_impl(scope, impl_for);
        Self::impl_read_fn(Self::new_read_fn(serializable_implementation), definition);
        Self::impl_write_fn(Self::new_write_fn(serializable_implementation), definition);
    }
}

impl UperGenerator {
    const CODEC: &'static str = "Uper";

    fn new_uper_serializable_impl<'a>(scope: &'a mut Scope, impl_for: &str) -> &'a mut Impl {
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
                Self::impl_read_fn_for_enumerated(function, name, &variants[..]);
            }
            Definition::Choice(name, variants) => {
                Self::impl_read_fn_for_choice(function, name, &variants[..]);
            }
        };
    }

    fn impl_read_fn_for_sequence_of(function: &mut Function, _name: &str, aliased: &Asn) {
        function.line("let mut me = Self::default();");
        function.line("let len = reader.read_length_determinant()?;");
        let mut block_for = Block::new("for _ in 0..len");
        match aliased {
            Asn::Boolean => block_for.line("me.values.push(reader.read_bit()?);"),
            Asn::Integer(_) => block_for.line(format!(
                "me.values.push(reader.read_int((Self::value_min() as i64, Self::value_max() as i64))? as {});",
                aliased.clone().into_rust().to_string(),
            )),
            Asn::UnsignedMaxInteger => {
                block_for.line("me.values.push(reader.read_int_max()?);")
            }
            Asn::UTF8String => {
                block_for.line(format!("me.values.push(reader.read_utf8_string()?);"))
            }
            Asn::OctetString => {
                block_for.line("me.values.push(reader.read_octet_string(None)?);")
            }
            Asn::TypeReference(custom) => block_for
                .line(format!("me.values.push({}::read_uper(reader)?);", custom)),
        };
        function.push_block(block_for);
        function.line("Ok(me)");
    }

    fn impl_read_fn_for_sequence(function: &mut Function, _name: &str, fields: &[Field]) {
        function.line("let mut me = Self::default();");

        // bitmask for optional fields
        for field in fields.iter() {
            if field.optional {
                function.line(format!(
                    "let {} = reader.read_bit()?;",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ));
            }
        }
        for field in fields.iter() {
            let line = format!(
                "me.{} = {}",
                RustCodeGenerator::rust_field_name(&field.name, true),
                Self::read_field(&field.name, &field.role, field.optional)
            );
            if field.optional {
                let mut block_if = Block::new(&format!(
                    "if {}",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ));
                block_if.line(line);
                let mut block_else = Block::new("else");
                block_else.line(format!(
                    "me.{} = None;",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ));
                function.push_block(block_if);
                function.push_block(block_else);
            } else {
                function.line(line);
            }
        }
        function.line("Ok(me)");
    }

    fn impl_read_fn_for_enumerated(function: &mut Function, name: &str, variants: &[String]) {
        function.line(format!(
            "let id = reader.read_int((0, {}))?;",
            variants.len() - 1
        ));
        let mut block_match = Block::new("match id");
        for (i, variant) in variants.iter().enumerate() {
            block_match.line(format!(
                "{} => Ok({}::{}),",
                i,
                name,
                RustCodeGenerator::rust_variant_name(&variant),
            ));
        }
        block_match.line(format!(
            "_ => Err(UperError::ValueNotInRange(id, 0, {}))",
            variants.len() - 1
        ));
        function.push_block(block_match);
    }

    fn impl_read_fn_for_choice(function: &mut Function, name: &str, variants: &[(String, Asn)]) {
        if variants.len() > 1 {
            function.line(&format!(
                "let variant = reader.read_int((0, {}))?;",
                variants.len() - 1
            ));
            let mut block = Block::new("match variant");
            for (i, (variant, role)) in variants.iter().enumerate() {
                let mut block_case = Block::new(&format!("{} =>", i));
                block_case.line(format!(
                    "let read = {}",
                    &Self::read_field(
                        if role.clone().into_rust().is_primitive() {
                            "*value"
                        } else {
                            "value"
                        },
                        role,
                        false,
                    )
                ));
                block_case.line(format!(
                    "Ok({}::{}(read))",
                    name,
                    RustCodeGenerator::rust_variant_name(variant),
                ));
                block.push_block(block_case);
            }
            block.line(format!(
                "_ => Err(UperError::ValueNotInRange(variant, 0, {}))",
                variants.len() - 1
            ));
            function.push_block(block);
        } else {
            function.line(&format!(
                "Ok({}::{}({}))",
                name,
                RustCodeGenerator::rust_variant_name(&variants[0].0),
                &Self::write_field(
                    if variants[0].1.clone().into_rust().is_primitive() {
                        "*value"
                    } else {
                        "value"
                    },
                    &variants[0].1,
                    variants[0].1.clone().into_rust().is_primitive(),
                    false,
                )
            ));
        }
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
            Definition::Choice(name, variants) => {
                Self::impl_write_fn_for_choice(function, name, &variants[..]);
            }
        };
        function.line("Ok(())");
    }

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

    fn impl_write_fn_for_enumeration(function: &mut Function, name: &str, variants: &[String]) {
        let mut block = Block::new("match self");
        for (i, variant) in variants.iter().enumerate() {
            block.line(format!(
                "{}::{} => writer.write_int({}, (0, {}))?,",
                name,
                RustCodeGenerator::rust_variant_name(&variant),
                i,
                variants.len() - 1
            ));
        }
        function.push_block(block);
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
    }
}
