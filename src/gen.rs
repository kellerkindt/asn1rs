use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;
use codegen::Type;

use model::Definition;
use model::Field;
use model::Model;
use model::Role;

const KEYWORDS: [&str; 9] = ["use", "mod", "const", "type", "pub", "enum", "struct", "impl", "trait"];

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
            files.push(Self::model_to_file(model)?);
        }
        Ok(files)
    }

    pub fn model_to_file(model: &Model) -> Result<(String, String), Error> {
        let file = {
            let mut string = Self::rust_module_name(&model.name);
            string.push_str(".rs");
            string
        };

        let mut scope = Scope::new();
        scope.import("super::buffer", "Error as WriteError");
        scope.import("super::buffer", "BitBuffer");

        for import in model.imports.iter() {
            let from = format!("super::{}", Self::rust_module_name(&import.from));
            for what in import.what.iter() {
                scope.import(&from, &what);
            }
        }

        for definition in model.definitions.iter() {
            let implementation = match definition {
                Definition::SequenceOf(name, role) => {
                    Self::new_struct(&mut scope, name).field("values", format!("Vec<{}>", Self::role_to_type(role)));
                    {
                        scope.new_impl(&name)
                            .impl_trait("::std::ops::Deref")
                            .associate_type("Target", format!("Vec<{}>", Self::role_to_type(role)))
                            .new_fn("deref")
                            .arg_ref_self()
                            .ret(&format!("&Vec<{}>", Self::role_to_type(role)))
                            .line(format!("&self.values"));
                    }
                    {
                        scope.new_impl(&name)
                            .impl_trait("::std::ops::DerefMut")
                            .new_fn("deref_mut")
                            .arg_mut_self()
                            .ret(&format!("&mut Vec<{}>", Self::role_to_type(role)))
                            .line(format!("&mut self.values"));
                    }
                    scope.new_impl(&name)
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
                    scope.new_impl(&name)
                }
                Definition::Enumerated(name, variants) => {
                    {
                        let mut enumeration = Self::new_enum(&mut scope, name);
                        for variant in variants.iter() {
                            enumeration.new_variant(&Self::rust_variant_name(&variant));
                        }
                    }
                    {
                        scope.new_impl(&name)
                            .impl_trait("Default")
                            .new_fn("default")
                            .ret(&name as &str)
                            .line(format!("{}::{}", name, Self::rust_variant_name(&variants[0])));
                    }
                    scope.new_impl(&name)
                }
            };
            match definition {
                Definition::SequenceOf(_name, aliased) => {
                    {
                        let mut block = Self::new_write_impl(implementation);
                        block.line("buffer.write_length_determinant(self.values.len())?;");
                        let mut block_for = Block::new("for value in self.values.iter()");
                        match aliased {
                            Role::Boolean => block_for.line("buffer.write_bit(value);"),
                            Role::Integer((lower, upper)) => block_for.line(format!("buffer.write_int(*value as i64, ({}, {}))?;", lower, upper)),
                            Role::Custom(custom) => block_for.line("value.write(buffer)?;")
                        };
                        block.push_block(block_for);
                        block.line("Ok(())");
                    }
                    {
                        implementation.new_fn("values")
                            .vis("pub")
                            .ret(format!("&Vec<{}>", Self::role_to_type(aliased)))
                            .arg_ref_self()
                            .line("&self.values");
                    }
                    {
                        implementation.new_fn("values_mut")
                            .vis("pub")
                            .ret(format!("&mut Vec<{}>", Self::role_to_type(aliased)))
                            .arg_mut_self()
                            .line("&mut self.values");
                    }
                    {
                        implementation.new_fn("set_values")
                            .vis("pub")
                            .arg_mut_self()
                            .arg("values",format!("Vec<{}>", Self::role_to_type(aliased)))
                            .line("self.values = values;");
                    }
                }
                Definition::Sequence(_name, fields) => {
                    {
                        let mut block = Self::new_write_impl(implementation);

                        // bitmask for optional fields
                        for field in fields.iter() {
                            if field.optional {
                                block.line(format!(
                                    "buffer.write_bit(self.{}.is_some());",
                                    Self::rust_field_name(&field.name),
                                ));
                            }
                        }

                        for field in fields.iter() {
                            let line = match field.role {
                                Role::Boolean => format!(
                                    "buffer.write_bit({}{});",
                                    if field.optional { "*" } else { "self." },
                                    Self::rust_field_name(&field.name),
                                ),
                                Role::Integer((lower, upper)) => format!(
                                    "buffer.write_int({}{} as i64, ({} as i64, {} as i64))?;",
                                    if field.optional { "*" } else { "self." },
                                    Self::rust_field_name(&field.name),
                                    lower,
                                    upper
                                ),
                                Role::Custom(ref _type) => format!(
                                    "{}{}.write(buffer)?;",
                                    if field.optional { "" } else { "self." },
                                    Self::rust_field_name(&field.name),
                                ),
                            };
                            if field.optional {
                                let mut b = Block::new(&format!(
                                    "if let Some(ref {}) = self.{}",
                                    Self::rust_field_name(&field.name),
                                    Self::rust_field_name(&field.name),
                                ));
                                b.line(line);
                                block.push_block(b);
                            } else {
                                block.line(line);
                            }
                        }

                        block.line("Ok(())");
                    }
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
                Definition::Enumerated(name, variants) => {
                    let mut block = Block::new("match self");
                    for (i, variant) in variants.iter().enumerate() {
                        block.line(format!(
                            "{}::{} => buffer.write_int({}, (0, {}))?,",
                            name,
                            Self::rust_variant_name(&variant),
                            i,
                            variants.len()
                        ));
                    }
                    Self::new_write_impl(implementation)
                        .push_block(block)
                        .line("Ok(())");
                }
            }
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

    fn new_write_impl<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        implementation
            .new_fn("write")
            .vis("pub")
            .arg_ref_self()
            .arg("buffer", "&mut BitBuffer")
            .ret("Result<(), WriteError>")
    }
}
