use codegen::Enum;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;
use codegen::Struct;

use model::Definition;
use model::Field;
use model::Model;
use model::Role;
use model::RustType;

use gen::Generator;

mod protobuf;
mod uper;

use self::protobuf::ProtobufGenerator;
use self::uper::UperGenerator;
use codegen::Block;

const KEYWORDS: [&str; 9] = [
    "use", "mod", "const", "type", "pub", "enum", "struct", "impl", "trait",
];

pub trait GeneratorSupplement {
    fn add_imports(&self, scope: &mut Scope);
    fn impl_supplement(&self, scope: &mut Scope, impl_for: &str, definition: &Definition);
}

#[derive(Debug, Default)]
pub struct RustCodeGenerator {
    models: Vec<Model>,
}

impl Generator for RustCodeGenerator {
    type Error = ();

    fn add_model(&mut self, model: Model) {
        self.models.push(model);
    }

    fn models(&self) -> &[Model] {
        &self.models[..]
    }

    fn models_mut(&mut self) -> &mut [Model] {
        &mut self.models[..]
    }

    fn to_string(&self) -> Result<Vec<(String, String)>, Self::Error> {
        let mut files = Vec::new();
        for model in self.models.iter() {
            files.push(RustCodeGenerator::model_to_file(
                model,
                &[&UperGenerator, &ProtobufGenerator],
            ));
        }
        Ok(files)
    }
}

impl RustCodeGenerator {
    pub fn model_to_file(model: &Model, generators: &[&GeneratorSupplement]) -> (String, String) {
        let file = {
            let mut string = Self::rust_module_name(&model.name);
            string.push_str(".rs");
            string
        };

        let mut scope = Scope::new();
        generators.iter().for_each(|g| g.add_imports(&mut scope));

        for import in model.imports.iter() {
            let from = format!("super::{}", Self::rust_module_name(&import.from));
            for what in import.what.iter() {
                scope.import(&from, &what);
            }
        }

        for definition in model.definitions.iter() {
            Self::add_definition(&mut scope, definition);
            Self::impl_definition(&mut scope, definition);

            let name: String = match definition {
                Definition::SequenceOf(name, _role) => name.clone(),
                Definition::Sequence(name, _fields) => name.clone(),
                Definition::Enumerated(name, _variants) => name.clone(),
            };

            generators
                .iter()
                .for_each(|g| g.impl_supplement(&mut scope, &name, &definition));
        }

        (file, scope.to_string())
    }

    fn add_definition(scope: &mut Scope, definition: &Definition) {
        match definition {
            Definition::SequenceOf(name, aliased) => {
                Self::add_sequence_of(Self::new_struct(scope, name), name, aliased);
            }
            Definition::Sequence(name, fields) => {
                Self::add_sequence(Self::new_struct(scope, name), name, &fields[..]);
            }
            Definition::Enumerated(name, variants) => {
                Self::add_enumerated(Self::new_enum(scope, name), name, &variants[..]);
            }
        }
    }

    fn add_sequence_of(str_ct: &mut Struct, _name: &str, aliased: &Role) {
        str_ct.field(
            "values",
            format!("Vec<{}>", aliased.clone().into_rust().to_string()),
        );
    }

    fn add_sequence(str_ct: &mut Struct, _name: &str, fields: &[Field]) {
        for field in fields.iter() {
            str_ct.field(
                &Self::rust_field_name(&field.name, true),
                if field.optional {
                    format!("Option<{}>", field.role.clone().into_rust().to_string())
                } else {
                    field.role.clone().into_rust().to_string()
                },
            );
        }
    }

    fn add_enumerated(en_m: &mut Enum, _name: &str, variants: &[String]) {
        for variant in variants.iter() {
            en_m.new_variant(&Self::rust_variant_name(&variant));
        }
    }

    fn impl_definition(scope: &mut Scope, definition: &Definition) {
        match definition {
            Definition::SequenceOf(name, aliased) => {
                let rust_type = aliased.clone().into_rust();
                Self::impl_sequence_of(scope, name, &aliased);
                Self::impl_sequence_of_deref(scope, name, &rust_type);
                Self::impl_sequence_of_deref_mut(scope, name, &rust_type);
            }
            Definition::Sequence(name, fields) => {
                Self::impl_sequence(scope, name, &fields[..]);
            }
            Definition::Enumerated(name, variants) => {
                Self::impl_enumerated(scope, name, &variants[..]);
                Self::impl_enumerated_default(scope, name, &variants[..]);
            }
        }
    }

    fn impl_sequence_of_deref(scope: &mut Scope, name: &str, aliased: &RustType) {
        scope
            .new_impl(&name)
            .impl_trait("::std::ops::Deref")
            .associate_type("Target", format!("Vec<{}>", aliased.to_string()))
            .new_fn("deref")
            .arg_ref_self()
            .ret(&format!("&Vec<{}>", aliased.to_string()))
            .line(format!("&self.values"));
    }

    fn impl_sequence_of_deref_mut(scope: &mut Scope, name: &str, aliased: &RustType) {
        scope
            .new_impl(&name)
            .impl_trait("::std::ops::DerefMut")
            .new_fn("deref_mut")
            .arg_mut_self()
            .ret(&format!("&mut Vec<{}>", aliased.to_string()))
            .line(format!("&mut self.values"));
    }

    fn impl_sequence_of(scope: &mut Scope, name: &str, aliased: &Role) {
        let implementation = scope.new_impl(name);
        let rust_type = aliased.clone().into_rust().to_string();

        Self::add_sequence_of_values_fn(implementation, &rust_type);
        Self::add_sequence_of_values_mut_fn(implementation, &rust_type);
        Self::add_sequence_of_set_values_fn(implementation, &rust_type);

        Self::add_min_max_fn_if_applicable(implementation, "value", &aliased);
    }

    fn add_sequence_of_values_fn(implementation: &mut Impl, rust_type: &str) {
        implementation
            .new_fn("values")
            .vis("pub")
            .ret(format!("&Vec<{}>", rust_type))
            .arg_ref_self()
            .line("&self.values");
    }

    fn add_sequence_of_values_mut_fn(implementation: &mut Impl, rust_type: &str) {
        implementation
            .new_fn("values_mut")
            .vis("pub")
            .ret(format!("&mut Vec<{}>", rust_type))
            .arg_mut_self()
            .line("&mut self.values");
    }

    fn add_sequence_of_set_values_fn(implementation: &mut Impl, rust_type: &str) {
        implementation
            .new_fn("set_values")
            .vis("pub")
            .arg_mut_self()
            .arg("values", format!("Vec<{}>", rust_type))
            .line("self.values = values;");
    }

    fn impl_sequence(scope: &mut Scope, name: &str, fields: &[Field]) {
        let implementation = scope.new_impl(name);

        for field in fields.iter() {
            Self::impl_sequence_field_get(implementation, field);
            Self::impl_sequence_field_get_mut(implementation, field);
            Self::impl_sequence_field_set(implementation, field);

            Self::add_min_max_fn_if_applicable(implementation, &field.name, &field.role);
        }
    }

    fn impl_sequence_field_get(implementation: &mut Impl, field: &Field) {
        implementation
            .new_fn(&Self::rust_field_name(&field.name, true))
            .vis("pub")
            .arg_ref_self()
            .ret(if field.optional {
                format!("&Option<{}>", field.role.clone().into_rust().to_string())
            } else {
                format!("&{}", field.role.clone().into_rust().to_string())
            })
            .line(format!(
                "&self.{}",
                Self::rust_field_name(&field.name, true)
            ));
    }

    fn impl_sequence_field_get_mut(implementation: &mut Impl, field: &Field) {
        implementation
            .new_fn(&format!(
                "{}_mut",
                Self::rust_field_name(&field.name, false)
            ))
            .vis("pub")
            .arg_mut_self()
            .ret(if field.optional {
                format!(
                    "&mut Option<{}>",
                    field.role.clone().into_rust().to_string()
                )
            } else {
                format!("&mut {}", field.role.clone().into_rust().to_string())
            })
            .line(format!(
                "&mut self.{}",
                Self::rust_field_name(&field.name, true)
            ));
    }

    fn impl_sequence_field_set(implementation: &mut Impl, field: &Field) {
        implementation
            .new_fn(&format!(
                "set_{}",
                Self::rust_field_name(&field.name, false)
            ))
            .vis("pub")
            .arg_mut_self()
            .arg(
                "value",
                if field.optional {
                    format!("Option<{}>", field.role.clone().into_rust().to_string())
                } else {
                    field.role.clone().into_rust().to_string()
                },
            )
            .line(format!(
                "self.{} = value;",
                Self::rust_field_name(&field.name, true)
            ));
    }

    fn impl_enumerated_default(scope: &mut Scope, name: &str, variants: &[String]) {
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

    fn impl_enumerated(scope: &mut Scope, name: &str, variants: &[String]) {
        let implementation = scope.new_impl(name);

        Self::impl_enumerated_values_fn(implementation, &name, variants);
        Self::impl_enumerated_value_index_fn(implementation, &name, variants);
    }

    fn impl_enumerated_values_fn(implementation: &mut Impl, name: &str, variants: &[String]) {
        let values_fn = implementation
            .new_fn("variants")
            .vis("pub")
            .ret(format!("[Self; {}]", variants.len()))
            .line("[");

        for variant in variants {
            values_fn.line(format!("{}::{},", name, Self::rust_variant_name(variant)));
        }
        values_fn.line("]");
    }

    fn impl_enumerated_value_index_fn(implementation: &mut Impl, name: &str, variants: &[String]) {
        let ordinal_fn = implementation
            .new_fn("value_index")
            .arg_ref_self()
            .vis("pub")
            .ret("usize");

        let mut block = Block::new("match self");
        variants.iter().enumerate().for_each(|(ordinal, variant)| {
            block.line(format!(
                "{}::{} => {},",
                name,
                Self::rust_variant_name(variant),
                ordinal
            ));
        });

        ordinal_fn.push_block(block);
    }

    fn add_min_max_fn_if_applicable(implementation: &mut Impl, name: &str, role: &Role) {
        let min_max = match role {
            Role::Boolean => None,
            Role::Integer((lower, upper)) => Some((*lower, *upper)),
            Role::UnsignedMaxInteger => Some((0, ::std::i64::MAX)),
            Role::UTF8String => None,
            Role::OctetString => None,
            Role::Custom(_) => None,
        };

        if let Some((min, max)) = min_max {
            implementation
                .new_fn(&format!("{}_min", Self::rust_field_name(name, false)))
                .vis("pub")
                .ret(&role.clone().into_rust().to_string())
                .line(format!("{}", min));
            implementation
                .new_fn(&format!("{}_max", Self::rust_field_name(name, false)))
                .vis("pub")
                .ret(&role.clone().into_rust().to_string())
                .line(format!("{}", max));
        }
    }

    fn rust_field_name(name: &str, check_for_keywords: bool) -> String {
        let mut name = name.replace("-", "_");
        if check_for_keywords {
            for keyword in KEYWORDS.iter() {
                if keyword.eq(&name) {
                    name.push_str("_");
                    return name;
                }
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

    fn new_struct<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Struct {
        scope
            .new_struct(name)
            .vis("pub")
            .derive("Default")
            .derive("Debug")
            .derive("Clone")
            .derive("PartialEq")
    }

    fn new_enum<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Enum {
        scope
            .new_enum(name)
            .vis("pub")
            .derive("Debug")
            .derive("Clone")
            .derive("Copy")
            .derive("PartialEq")
            .derive("PartialOrd")
    }

    fn new_serializable_impl<'a>(
        scope: &'a mut Scope,
        impl_for: &str,
        codec: &str,
    ) -> &'a mut Impl {
        scope.new_impl(impl_for).impl_trait(codec)
    }

    fn new_read_fn<'a>(implementation: &'a mut Impl, codec: &str) -> &'a mut Function {
        implementation
            .new_fn(&format!("read_{}", codec.to_lowercase()))
            .arg("reader", format!("&mut {}Reader", codec))
            .ret(format!("Result<Self, {}Error>", codec))
            .bound("Self", "Sized")
    }

    fn new_write_fn<'a>(implementation: &'a mut Impl, codec: &str) -> &'a mut Function {
        implementation
            .new_fn(&format!("write_{}", codec.to_lowercase()))
            .arg_ref_self()
            .arg("writer", format!("&mut {}Writer", codec))
            .ret(format!("Result<(), {}Error>", codec))
    }
}
