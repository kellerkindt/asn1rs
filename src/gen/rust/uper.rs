use codegen::Block;
use codegen::Enum;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;
use codegen::Struct;

use model::Definition;
use model::Field;
use model::Model;
use model::ProtobufType;
use model::Role;
use model::RustType;

use gen::rust::RustCodeGenerator;
use gen::rust::GeneratorSupplement;

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

    fn generate_implementations(&self, scope: &mut Scope, impl_for: &str, definition: &Definition) {
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
                Self::impl_read_fn_for_enumeration(function, name, &variants[..]);
            }
        };
    }

    fn impl_read_fn_for_sequence_of(function: &mut Function, name: &String, aliased: &Role) {
        function.line("let mut me = Self::default();");
        function.line("let len = reader.read_length_determinant()?;");
        let mut block_for = Block::new("for _ in 0..len");
        match aliased {
            Role::Boolean => block_for.line("me.values.push(reader.read_bit()?);"),
            Role::Integer(_) => block_for.line(format!(
                "me.values.push(reader.read_int((Self::value_min() as i64, Self::value_max() as i64))? as {});",
                aliased.clone().into_rust().to_string(),
            )),
            Role::UnsignedMaxInteger => {
                block_for.line("me.values.push(reader.read_int_max()?);")
            }
            Role::Custom(custom) => block_for
                .line(format!("me.values.push({}::read_uper(reader)?);", custom)),
            Role::UTF8String => {
                block_for.line(format!("me.values.push(reader.read_utf8_string()?);"))
            }
        };
        function.push_block(block_for);
        function.line("Ok(me)");
    }

    fn impl_read_fn_for_sequence(function: &mut Function, name: &String, fields: &[Field]) {
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
            let line = match field.role {
                Role::Boolean => format!(
                    "me.{} = {}reader.read_bit()?{};",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                    if field.optional { "Some(" } else { "" },
                    if field.optional { ")" } else { "" },
                ),
                Role::Integer(_) => format!(
                    "me.{} = {}reader.read_int((Self::{}_min() as i64, Self::{}_max() as i64))? as {}{};",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                    if field.optional { "Some(" } else { "" },
                    RustCodeGenerator::rust_field_name(&field.name, false),
                    RustCodeGenerator::rust_field_name(&field.name, false),
                    field.role.clone().into_rust().to_string(),
                    if field.optional { ")" } else { "" },
                ),
                Role::UnsignedMaxInteger => format!(
                    "me.{} = {}reader.read_int_max()?{};",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                    if field.optional { "Some(" } else { "" },
                    if field.optional { ")" } else { "" },
                ),
                Role::Custom(ref _type) => format!(
                    "me.{} = {}{}::read_uper(reader)?{};",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                    if field.optional { "Some(" } else { "" },
                    field.role.clone().into_rust().to_string(),
                    if field.optional { ")" } else { "" },
                ),
                Role::UTF8String => format!(
                    "me.{} = reader.read_utf8_string()?;",
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ),
            };
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

    fn impl_read_fn_for_enumeration(function: &mut Function, name: &String, variants: &[String]) {
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
            variants.len()
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

    fn impl_write_fn_for_sequence_of(function: &mut Function, name: &String, aliased: &Role) {
        function.line("writer.write_length_determinant(self.values.len())?;");
        let mut block_for = Block::new("for value in self.values.iter()");
        match aliased {
            Role::Boolean => block_for.line("writer.write_bit(value)?;"),
            Role::Integer(_) => block_for.line(format!(
                "writer.write_int(*value as i64, (Self::value_min() as i64, Self::value_max() as i64))?;"
            )),
            Role::UnsignedMaxInteger => {
                block_for.line("writer.write_int_max(*value)?;")
            }
            Role::Custom(_custom) => block_for.line("value.write_uper(writer)?;"),
            Role::UTF8String => block_for.line("writer.write_utf8_string(&value)?;"),
        };
        function.push_block(block_for);
    }

    fn impl_write_fn_for_sequence(function: &mut Function, name: &String, fields: &[Field]) {
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
            let line = match field.role {
                Role::Boolean => format!(
                    "writer.write_bit({}{})?;",
                    if field.optional { "*" } else { "self." },
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ),
                Role::Integer(_) => format!(
                    "writer.write_int({}{} as i64, (Self::{}_min() as i64, Self::{}_max() as i64))?;",
                    if field.optional { "*" } else { "self." },
                    RustCodeGenerator::rust_field_name(&field.name, true),
                    RustCodeGenerator::rust_field_name(&field.name, false),
                    RustCodeGenerator::rust_field_name(&field.name, false),
                ),
                Role::UnsignedMaxInteger => format!(
                    "writer.write_int_max({}{})?;",
                    if field.optional { "*" } else { "self." },
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ),
                Role::Custom(ref _type) => format!(
                    "{}{}.write_uper(writer)?;",
                    if field.optional { "" } else { "self." },
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ),
                Role::UTF8String => format!(
                    "writer.write_utf8_string(&{}{})?;",
                    if field.optional { "" } else { "self." },
                    RustCodeGenerator::rust_field_name(&field.name, true),
                ),
            };
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

    fn impl_write_fn_for_enumeration(function: &mut Function, name: &String, variants: &[String]) {
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
}