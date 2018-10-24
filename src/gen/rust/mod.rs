mod protobuf;
mod uper;

use codegen::Block;
use codegen::Enum;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;
use codegen::Struct;

use model::Definition;
use model::Model;
use model::Range;
use model::Rust;
use model::RustType;

use gen::Generator;

use self::protobuf::ProtobufSerializer;
use self::uper::UperSerializer;

const KEYWORDS: [&str; 9] = [
    "use", "mod", "const", "type", "pub", "enum", "struct", "impl", "trait",
];

pub trait GeneratorSupplement<T> {
    fn add_imports(&self, scope: &mut Scope);
    fn impl_supplement(&self, scope: &mut Scope, definition: &Definition<T>);
}

#[derive(Debug, Default)]
pub struct RustCodeGenerator {
    models: Vec<Model<Rust>>,
}

impl Generator<Rust> for RustCodeGenerator {
    type Error = ();

    fn add_model(&mut self, model: Model<Rust>) {
        self.models.push(model);
    }

    fn models(&self) -> &[Model<Rust>] {
        &self.models[..]
    }

    fn models_mut(&mut self) -> &mut [Model<Rust>] {
        &mut self.models[..]
    }

    fn to_string(&self) -> Result<Vec<(String, String)>, Self::Error> {
        let mut files = Vec::new();
        for model in &self.models {
            files.push(RustCodeGenerator::model_to_file(
                model,
                &[&UperSerializer, &ProtobufSerializer],
            ));
        }
        Ok(files)
    }
}

impl RustCodeGenerator {
    pub fn model_to_file(
        model: &Model<Rust>,
        generators: &[&GeneratorSupplement<Rust>],
    ) -> (String, String) {
        let file = {
            let mut string = Self::rust_module_name(&model.name);
            string.push_str(".rs");
            string
        };

        let mut scope = Scope::new();
        generators.iter().for_each(|g| g.add_imports(&mut scope));

        for import in &model.imports {
            let from = format!("super::{}", &Self::rust_module_name(&import.from));
            for what in &import.what {
                scope.import(&from, &what);
            }
        }

        for definition in &model.definitions {
            Self::add_definition(&mut scope, definition);
            Self::impl_definition(&mut scope, definition);

            generators
                .iter()
                .for_each(|g| g.impl_supplement(&mut scope, &definition));
        }

        (file, scope.to_string())
    }

    fn add_definition(scope: &mut Scope, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::Struct(fields) => Self::add_struct(Self::new_struct(scope, name), name, fields),
            Rust::Enum(variants) => {
                Self::add_enum(Self::new_enum(scope, name, true), name, variants)
            }
            Rust::DataEnum(variants) => {
                Self::add_data_enum(Self::new_enum(scope, name, false), name, variants)
            }
            Rust::TupleStruct(inner) => {
                Self::add_tuple_struct(Self::new_struct(scope, name), name, inner)
            }
        }
    }

    fn add_struct(str_ct: &mut Struct, _name: &str, fields: &[(String, RustType)]) {
        for (field_name, field_type) in fields.iter() {
            str_ct.field(
                &Self::rust_field_name(field_name, true),
                field_type.to_string(),
            );
        }
    }

    fn add_enum(en_m: &mut Enum, _name: &str, variants: &[String]) {
        for variant in variants.iter() {
            en_m.new_variant(&Self::rust_variant_name(&variant));
        }
    }

    fn add_data_enum(en_m: &mut Enum, _name: &str, variants: &[(String, RustType)]) {
        for (variant, rust_type) in variants.iter() {
            en_m.new_variant(&format!(
                "{}({})",
                Self::rust_variant_name(variant),
                rust_type.to_string(),
            ));
        }
    }

    fn add_tuple_struct(str_ct: &mut Struct, _name: &str, inner: &RustType) {
        str_ct.tuple_field(inner.to_string());
    }

    fn impl_definition(scope: &mut Scope, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::Struct(fields) => {
                Self::impl_struct(scope, name, fields);
            }
            Rust::Enum(variants) => {
                Self::impl_enum(scope, name, variants);
                Self::impl_default_default(scope, name, variants);
            }
            Rust::DataEnum(variants) => {
                Self::impl_data_enum(scope, name, variants);
                Self::impl_data_enum_default(scope, name, variants);
            }
            Rust::TupleStruct(inner) => {
                Self::impl_tuple_struct(scope, name, inner);
                Self::impl_tuple_struct_deref(scope, name, inner);
                Self::impl_tuple_struct_deref_mut(scope, name, inner);
            }
        }
    }

    fn impl_tuple_struct_deref(scope: &mut Scope, name: &str, rust: &RustType) {
        scope
            .new_impl(&name)
            .impl_trait("::std::ops::Deref")
            .associate_type("Target", rust.to_string())
            .new_fn("deref")
            .arg_ref_self()
            .ret(&format!("&{}", rust.to_string()))
            .line("&self.0".to_string());
    }

    fn impl_tuple_struct_deref_mut(scope: &mut Scope, name: &str, rust: &RustType) {
        scope
            .new_impl(&name)
            .impl_trait("::std::ops::DerefMut")
            .new_fn("deref_mut")
            .arg_mut_self()
            .ret(&format!("&mut {}", rust.to_string()))
            .line("&mut self.0".to_string());
    }

    fn impl_tuple_struct(scope: &mut Scope, name: &str, rust: &RustType) {
        let implementation = scope.new_impl(name);
        Self::add_min_max_fn_if_applicable(implementation, "value", &rust);
    }

    fn impl_struct(scope: &mut Scope, name: &str, fields: &[(String, RustType)]) {
        let implementation = scope.new_impl(name);

        for (field_name, field_type) in fields.iter() {
            Self::impl_struct_field_get(implementation, field_name, field_type);
            Self::impl_struct_field_get_mut(implementation, field_name, field_type);
            Self::impl_struct_field_set(implementation, field_name, field_type);

            Self::add_min_max_fn_if_applicable(implementation, field_name, field_type);
        }
    }

    fn impl_struct_field_get(implementation: &mut Impl, field_name: &str, field_type: &RustType) {
        implementation
            .new_fn(&Self::rust_field_name(field_name, true))
            .vis("pub")
            .arg_ref_self()
            .ret(format!("&{}", field_type.to_string()))
            .line(format!("&self.{}", Self::rust_field_name(field_name, true)));
    }

    fn impl_struct_field_get_mut(
        implementation: &mut Impl,
        field_name: &str,
        field_type: &RustType,
    ) {
        implementation
            .new_fn(&format!("{}_mut", field_name))
            .vis("pub")
            .arg_mut_self()
            .ret(format!("&mut {}", field_type.to_string()))
            .line(format!(
                "&mut self.{}",
                Self::rust_field_name(field_name, true)
            ));
    }

    fn impl_struct_field_set(implementation: &mut Impl, field_name: &str, field_type: &RustType) {
        implementation
            .new_fn(&format!("set_{}", field_name))
            .vis("pub")
            .arg_mut_self()
            .arg("value", field_type.to_string())
            .line(format!(
                "self.{} = value;",
                Self::rust_field_name(field_name, true)
            ));
    }

    fn impl_default_default(scope: &mut Scope, name: &str, variants: &[String]) {
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

    fn impl_enum(scope: &mut Scope, name: &str, variants: &[String]) {
        let implementation = scope.new_impl(name);

        Self::impl_enum_values_fn(implementation, &name, variants);
        Self::impl_enum_value_index_fn(implementation, &name, variants);
    }

    fn impl_enum_values_fn(implementation: &mut Impl, name: &str, variants: &[String]) {
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

    fn impl_enum_value_index_fn(implementation: &mut Impl, name: &str, variants: &[String]) {
        let ordinal_fn = implementation
            .new_fn("value_index")
            .arg_self()
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

    fn impl_data_enum(scope: &mut Scope, name: &str, variants: &[(String, RustType)]) {
        let implementation = scope.new_impl(name);

        Self::impl_data_enum_values_fn(implementation, &name, variants);
        Self::impl_data_enum_value_index_fn(implementation, &name, variants);
    }

    fn impl_data_enum_values_fn(
        implementation: &mut Impl,
        name: &str,
        variants: &[(String, RustType)],
    ) {
        let values_fn = implementation
            .new_fn("variants")
            .vis("pub")
            .ret(format!("[Self; {}]", variants.len()))
            .line("[");

        for (variant, _) in variants {
            values_fn.line(format!(
                "{}::{}(Default::default()),",
                name,
                Self::rust_variant_name(variant)
            ));
        }
        values_fn.line("]");
    }

    fn impl_data_enum_value_index_fn(
        implementation: &mut Impl,
        name: &str,
        variants: &[(String, RustType)],
    ) {
        let ordinal_fn = implementation
            .new_fn("value_index")
            .arg_ref_self()
            .vis("pub")
            .ret("usize");

        let mut block = Block::new("match self");
        variants
            .iter()
            .enumerate()
            .for_each(|(ordinal, (variant, _))| {
                block.line(format!(
                    "{}::{}(_) => {},",
                    name,
                    Self::rust_variant_name(variant),
                    ordinal
                ));
            });

        ordinal_fn.push_block(block);
    }

    fn impl_data_enum_default(scope: &mut Scope, name: &str, variants: &[(String, RustType)]) {
        scope
            .new_impl(&name)
            .impl_trait("Default")
            .new_fn("default")
            .ret(&name as &str)
            .line(format!(
                "{}::{}(Default::default())",
                name,
                Self::rust_variant_name(&variants[0].0)
            ));
    }

    fn add_min_max_fn_if_applicable(
        implementation: &mut Impl,
        field_name: &str,
        field_type: &RustType,
    ) {
        if let Some(Range(min, max)) = field_type.integer_range_str() {
            implementation
                .new_fn(&format!("{}_min", field_name))
                .vis("pub")
                .ret(&field_type.to_inner_type_string())
                .line(&Self::format_number_nicely(&min));
            implementation
                .new_fn(&format!("{}_max", field_name))
                .vis("pub")
                .ret(&field_type.to_inner_type_string())
                .line(&Self::format_number_nicely(&max));
        }
    }

    fn format_number_nicely(string: &str) -> String {
        let mut out = String::with_capacity(string.len() * 2);
        let mut pos = (3 - string.len() % 3) % 3;
        for char in string.chars() {
            out.push(char);
            pos = (pos + 1) % 3;
            if pos == 0 && char.is_numeric() {
                out.push('_');
            }
        }
        let len = out.len();
        out.remove(len - 1);
        out
    }

    pub fn rust_field_name(name: &str, check_for_keywords: bool) -> String {
        let mut name = name.replace("-", "_");
        if check_for_keywords {
            for keyword in &KEYWORDS {
                if keyword.eq(&name) {
                    name.push_str("_");
                    return name;
                }
            }
        }
        name
    }

    pub fn rust_variant_name(name: &str) -> String {
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

    pub fn rust_module_name(name: &str) -> String {
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

    fn new_enum<'a>(scope: &'a mut Scope, name: &str, c_enum: bool) -> &'a mut Enum {
        let en_m = scope
            .new_enum(name)
            .vis("pub")
            .derive("Debug")
            .derive("Clone")
            .derive("PartialEq");
        if c_enum {
            en_m.derive("Copy").derive("PartialOrd");
        }
        en_m
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
