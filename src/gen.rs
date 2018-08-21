use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;

use model::Definition;
use model::Model;
use model::Role;

const KEYWORDS: [&str; 9] = [
    "use", "mod", "const", "type", "pub", "enum", "struct", "impl", "trait",
];

#[derive(Debug)]
pub enum Error {}

pub struct Generator {
    models: Vec<Model>,
}

impl Generator {
    pub fn new() -> Generator {
        Generator { models: Vec::new() }
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.push(model);
    }

    pub fn to_string(&self) -> Result<Vec<(String, String)>, Error> {
        let mut files = Vec::new();
        for model in self.models.iter() {
            files.push(Self::model_to_file(model, &[&UperGenerator])?);
        }
        Ok(files)
    }

    pub fn model_to_file(
        model: &Model,
        generators: &[&SerializableGenerator],
    ) -> Result<(String, String), Error> {
        let file = {
            let mut string = Self::rust_module_name(&model.name);
            string.push_str(".rs");
            string
        };

        let mut scope = Scope::new();
        scope.import("asn1c::io", "Codec");
        scope.import("asn1c::io", "Serializable");
        generators.iter().for_each(|g| g.add_imports(&mut scope));

        for import in model.imports.iter() {
            let from = format!("super::{}", Self::rust_module_name(&import.from));
            for what in import.what.iter() {
                scope.import(&from, &what);
            }
        }

        for definition in model.definitions.iter() {
            let name: String = match definition {
                Definition::SequenceOf(name, role) => {
                    Self::new_struct(&mut scope, name)
                        .field("values", format!("Vec<{}>", Self::role_to_type(role)));
                    {
                        scope
                            .new_impl(&name)
                            .impl_trait("::std::ops::Deref")
                            .associate_type("Target", format!("Vec<{}>", Self::role_to_type(role)))
                            .new_fn("deref")
                            .arg_ref_self()
                            .ret(&format!("&Vec<{}>", Self::role_to_type(role)))
                            .line(format!("&self.values"));
                    }
                    {
                        scope
                            .new_impl(&name)
                            .impl_trait("::std::ops::DerefMut")
                            .new_fn("deref_mut")
                            .arg_mut_self()
                            .ret(&format!("&mut Vec<{}>", Self::role_to_type(role)))
                            .line(format!("&mut self.values"));
                    }
                    {
                        let implementation = scope.new_impl(&name);
                        {
                            implementation
                                .new_fn("values")
                                .vis("pub")
                                .ret(format!("&Vec<{}>", Self::role_to_type(role)))
                                .arg_ref_self()
                                .line("&self.values");
                        }
                        {
                            implementation
                                .new_fn("values_mut")
                                .vis("pub")
                                .ret(format!("&mut Vec<{}>", Self::role_to_type(role)))
                                .arg_mut_self()
                                .line("&mut self.values");
                        }
                        {
                            implementation
                                .new_fn("set_values")
                                .vis("pub")
                                .arg_mut_self()
                                .arg("values", format!("Vec<{}>", Self::role_to_type(role)))
                                .line("self.values = values;");
                        }
                    }
                    name.clone()
                }
                Definition::Sequence(name, fields) => {
                    {
                        let mut new_struct = Self::new_struct(&mut scope, name);
                        for field in fields.iter() {
                            new_struct.field(
                                &Self::rust_field_name(&field.name),
                                if field.optional {
                                    format!("Option<{}>", Self::role_to_type(&field.role))
                                } else {
                                    Self::role_to_type(&field.role)
                                },
                            );
                        }
                    }
                    {
                        let implementation = scope.new_impl(name);

                        for field in fields.iter() {
                            implementation
                                .new_fn(&Self::rust_field_name(&field.name))
                                .vis("pub")
                                .arg_ref_self()
                                .ret(if field.optional {
                                    format!("&Option<{}>", Self::role_to_type(&field.role))
                                } else {
                                    format!("&{}", Self::role_to_type(&field.role))
                                })
                                .line(format!("&self.{}", Self::rust_field_name(&field.name)));

                            implementation
                                .new_fn(&format!("{}_mut", Self::rust_field_name(&field.name)))
                                .vis("pub")
                                .arg_mut_self()
                                .ret(if field.optional {
                                    format!("&mut Option<{}>", Self::role_to_type(&field.role))
                                } else {
                                    format!("&mut {}", Self::role_to_type(&field.role))
                                })
                                .line(format!("&mut self.{}", Self::rust_field_name(&field.name)));

                            implementation
                                .new_fn(&format!("set_{}", Self::rust_field_name(&field.name)))
                                .vis("pub")
                                .arg_mut_self()
                                .arg(
                                    "value",
                                    if field.optional {
                                        format!("Option<{}>", Self::role_to_type(&field.role))
                                    } else {
                                        Self::role_to_type(&field.role)
                                    },
                                )
                                .line(format!(
                                    "self.{} = value;",
                                    Self::rust_field_name(&field.name)
                                ));
                        }
                    }
                    name.clone()
                }
                Definition::Enumerated(name, variants) => {
                    {
                        let mut enumeration = Self::new_enum(&mut scope, name);
                        for variant in variants.iter() {
                            enumeration.new_variant(&Self::rust_variant_name(&variant));
                        }
                    }
                    {
                        scope
                            .new_impl(&name)
                            .impl_trait("Default")
                            .new_fn("default")
                            .ret(&name as &str)
                            .line(format!(
                                "{}::{}",
                                name,
                                Self::rust_variant_name(&variants[0])
                            ));
                    }
                    name.clone()
                }
            };
            generators
                .iter()
                .for_each(|g| g.generate_serializable_impl(&mut scope, &name, &definition));
        }

        Ok((file, scope.to_string()))
    }

    fn role_to_type(role: &Role) -> String {
        let type_name = match role {
            Role::Boolean => "bool".into(),
            Role::Integer((lower, upper)) => match lower.abs().max(*upper) {
                0x00_00_00_00__00_00_00_00...0x00_00_00_00__00_00_00_7F => "i8".into(),
                0x00_00_00_00__00_00_00_00...0x00_00_00_00__00_00_7F_FF => "i16".into(),
                0x00_00_00_00__00_00_00_00...0x00_00_00_00__7F_FF_FF_FF => "i32".into(),
                _ => "i64".into(),
            },
            Role::Custom(name) => name.clone(),
            Role::UTF8String => "String".into(),
        };
        type_name
    }

    fn rust_field_name(name: &str) -> String {
        let mut name = name.replace("-", "_");
        for keyword in KEYWORDS.iter() {
            if keyword.eq(&name) {
                name.push_str("_");
                return name;
            }
        }
        name
    }

    fn rust_variant_name(name: &str) -> String {
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

    fn rust_module_name(name: &str) -> String {
        let mut out = String::new();
        let mut prev_lowered = false;
        let mut chars = name.chars().peekable();
        while let Some(c) = chars.next() {
            let mut lowered = false;
            if c.is_uppercase() {
                if !out.is_empty() {
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
        }
        out
    }

    fn new_struct<'a>(scope: &'a mut Scope, name: &str) -> &'a mut ::codegen::Struct {
        scope
            .new_struct(name)
            .vis("pub")
            .derive("Default")
            .derive("Debug")
            .derive("Clone")
            .derive("PartialEq")
    }

    fn new_enum<'a>(scope: &'a mut Scope, name: &str) -> &'a mut ::codegen::Enum {
        scope
            .new_enum(name)
            .vis("pub")
            .derive("Debug")
            .derive("Clone")
            .derive("Copy")
            .derive("PartialEq")
            .derive("PartialOrd")
    }

    fn new_uper_serializable_impl<'a>(scope: &'a mut Scope, impl_for: &str, codec: &str) -> &'a mut Impl {
        scope.new_impl(impl_for).impl_trait(format!("Serializable<{}>", codec))
    }

    fn new_read_fn<'a>(implementation: &'a mut Impl, codec: &str) -> &'a mut Function {
        implementation
            .new_fn("read")
            .arg("reader", format!("&mut <{} as Codec>::Reader", codec))
            .ret(format!("Result<Self, {}Error>", codec))
    }

    fn new_write_fn<'a>(implementation: &'a mut Impl, codec: &str) -> &'a mut Function {
        implementation
            .new_fn("write")
            .arg_ref_self()
            .arg("writer", format!("&mut <{} as Codec>::Writer", codec))
            .ret(format!("Result<(), {}Error>", codec))
    }
}

trait SerializableGenerator {
    fn add_imports(&self, scope: &mut Scope);
    fn generate_serializable_impl(
        &self,
        scope: &mut Scope,
        impl_for: &str,
        definition: &Definition,
    );
}

pub struct UperGenerator;
impl SerializableGenerator for UperGenerator {
    fn add_imports(&self, scope: &mut Scope) {
        Self::add_imports(scope)
    }

    fn generate_serializable_impl(
        &self,
        scope: &mut Scope,
        impl_for: &str,
        definition: &Definition,
    ) {
        Self::generate_serializable_impl(scope, impl_for, definition)
    }
}

impl UperGenerator {
    const CODEC: &'static str = "Uper";

    fn new_uper_serializable_impl<'a>(scope: &'a mut Scope, impl_for: &str) -> &'a mut Impl {
        Generator::new_uper_serializable_impl(scope, impl_for, Self::CODEC)
    }

    fn new_read_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        Generator::new_read_fn(implementation, Self::CODEC)
    }

    fn new_write_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        Generator::new_write_fn(implementation, Self::CODEC)
    }

    fn add_imports(scope: &mut Scope) {
        scope.import("asn1c::io::uper", "Uper");
        scope.import("asn1c::io::uper", &format!("Error as {}Error", Self::CODEC));
    }

    fn generate_serializable_impl(scope: &mut Scope, impl_for: &str, definition: &Definition) {
        let serializable_implementation = Self::new_uper_serializable_impl(scope, impl_for);
        match definition {
            Definition::SequenceOf(_name, aliased) => {
                {
                    let mut block = Self::new_write_fn(serializable_implementation);
                    block.line("writer.write_length_determinant(self.values.len())?;");
                    let mut block_for = Block::new("for value in self.values.iter()");
                    match aliased {
                        Role::Boolean => block_for.line("writer.write_bit(value)?;"),
                        Role::Integer((lower, upper)) => block_for.line(format!(
                            "writer.write_int(*value as i64, ({}, {}))?;",
                            lower, upper
                        )),
                        Role::Custom(_custom) => block_for.line("value.write(writer)?;"),
                        Role::UTF8String => block_for.line("writer.write_utf8_string(&value)?;"),
                    };
                    block.push_block(block_for);
                    block.line("Ok(())");
                }
                {
                    let mut block = Self::new_read_fn(serializable_implementation);
                    block.line("let mut me = Self::default();");
                    block.line("let len = reader.read_length_determinant()?;");
                    let mut block_for = Block::new("for _ in 0..len");
                    match aliased {
                        Role::Boolean => block_for.line("me.values.push(reader.read_bit()?);"),
                        Role::Integer((lower, upper)) => block_for.line(format!(
                            "me.values.push(reader.read_int(({}, {}))? as {});",
                            lower,
                            upper,
                            Generator::role_to_type(aliased)
                        )),
                        Role::Custom(custom) => {
                            block_for.line(format!("me.values.push({}::read(reader)?);", custom))
                        }
                        Role::UTF8String => {
                            block_for.line(format!("me.values.push(reader.read_utf8_string()?);"))
                        }
                    };
                    block.push_block(block_for);
                    block.line("Ok(me)");
                }
            }
            Definition::Sequence(_name, fields) => {
                {
                    let block = Self::new_write_fn(serializable_implementation);

                    // bitmask for optional fields
                    for field in fields.iter() {
                        if field.optional {
                            block.line(format!(
                                "writer.write_bit(self.{}.is_some())?;",
                                Generator::rust_field_name(&field.name),
                            ));
                        }
                    }

                    for field in fields.iter() {
                        let line = match field.role {
                            Role::Boolean => format!(
                                "writer.write_bit({}{})?;",
                                if field.optional { "*" } else { "self." },
                                Generator::rust_field_name(&field.name),
                            ),
                            Role::Integer((lower, upper)) => format!(
                                "writer.write_int({}{} as i64, ({} as i64, {} as i64))?;",
                                if field.optional { "*" } else { "self." },
                                Generator::rust_field_name(&field.name),
                                lower,
                                upper
                            ),
                            Role::Custom(ref _type) => format!(
                                "{}{}.write(writer)?;",
                                if field.optional { "" } else { "self." },
                                Generator::rust_field_name(&field.name),
                            ),
                            Role::UTF8String => format!(
                                "writer.write_utf8_string(&{}{})?;",
                                if field.optional { "" } else { "self." },
                                Generator::rust_field_name(&field.name),
                            ),
                        };
                        if field.optional {
                            let mut b = Block::new(&format!(
                                "if let Some(ref {}) = self.{}",
                                Generator::rust_field_name(&field.name),
                                Generator::rust_field_name(&field.name),
                            ));
                            b.line(line);
                            block.push_block(b);
                        } else {
                            block.line(line);
                        }
                    }

                    block.line("Ok(())");
                }
                {
                    let block = Self::new_read_fn(serializable_implementation);
                    block.line("let mut me = Self::default();");

                    // bitmask for optional fields
                    for field in fields.iter() {
                        if field.optional {
                            block.line(format!(
                                "let {} = reader.read_bit()?;",
                                Generator::rust_field_name(&field.name),
                            ));
                        }
                    }
                    for field in fields.iter() {
                        let line = match field.role {
                            Role::Boolean => format!(
                                "me.{} = {}reader.read_bit()?{};",
                                Generator::rust_field_name(&field.name),
                                if field.optional { "Some(" } else { "" },
                                if field.optional { ")" } else { "" },
                            ),
                            Role::Integer((lower, upper)) => format!(
                                "me.{} = {}reader.read_int(({} as i64, {} as i64))? as {}{};",
                                Generator::rust_field_name(&field.name),
                                if field.optional { "Some(" } else { "" },
                                lower,
                                upper,
                                Generator::role_to_type(&field.role),
                                if field.optional { ")" } else { "" },
                            ),
                            Role::Custom(ref _type) => format!(
                                "me.{} = {}{}::read(reader)?{};",
                                Generator::rust_field_name(&field.name),
                                if field.optional { "Some(" } else { "" },
                                Generator::role_to_type(&field.role),
                                if field.optional { ")" } else { "" },
                            ),
                            Role::UTF8String => format!(
                                "me.{} = reader.read_utf8_string()?;",
                                Generator::rust_field_name(&field.name),
                            ),
                        };
                        if field.optional {
                            let mut block_if = Block::new(&format!(
                                "if {}",
                                Generator::rust_field_name(&field.name),
                            ));
                            block_if.line(line);
                            let mut block_else = Block::new("else");
                            block_else.line(format!(
                                "me.{} = None;",
                                Generator::rust_field_name(&field.name),
                            ));
                            block.push_block(block_if);
                            block.push_block(block_else);
                        } else {
                            block.line(line);
                        }
                    }

                    block.line("Ok(me)");
                }
            }
            Definition::Enumerated(name, variants) => {
                {
                    let mut block = Block::new("match self");
                    for (i, variant) in variants.iter().enumerate() {
                        block.line(format!(
                            "{}::{} => writer.write_int({}, (0, {}))?,",
                            name,
                            Generator::rust_variant_name(&variant),
                            i,
                            variants.len() - 1
                        ));
                    }
                    Self::new_write_fn(serializable_implementation)
                        .push_block(block)
                        .line("Ok(())");
                }
                {
                    let mut block = Self::new_read_fn(serializable_implementation);
                    block.line(format!(
                        "let id = reader.read_int((0, {}))?;",
                        variants.len() - 1
                    ));
                    let mut block_match = Block::new("match id");
                    for (i, variant) in variants.iter().enumerate() {
                        block_match.line(format!(
                            "{} => Ok({}::{}),",
                            i,
                            name,
                            Generator::rust_variant_name(&variant),
                        ));
                    }
                    block_match.line(format!(
                        "_ => Err(UperError::ValueNotInRange(id, 0, {}))",
                        variants.len()
                    ));
                    block.push_block(block_match);
                }
            }
        }
    }
}
