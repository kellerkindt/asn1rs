use crate::asn::{Tag, TagProperty, Type as AsnType, Type};
use crate::generator::Generator;
use crate::model::rust::{DataEnum, Field};
use crate::model::rust::{EncodingOrdering, PlainEnum};
use crate::model::Rust;
use crate::model::RustType;
use crate::model::{Definition, Model};
use codegen::Block;
use codegen::Enum;
use codegen::Impl;
use codegen::Scope;
use codegen::Struct;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Display;

const KEYWORDS: [&str; 9] = [
    "use", "mod", "const", "type", "pub", "enum", "struct", "impl", "trait",
];

pub trait GeneratorSupplement<T> {
    fn add_imports(&self, scope: &mut Scope);
    fn impl_supplement(&self, scope: &mut Scope, definition: &Definition<T>);
    fn extend_impl_of_struct(&self, _name: &str, _impl_scope: &mut Impl, _fields: &[Field]) {}
    fn extend_impl_of_enum(&self, _name: &str, _impl_scope: &mut Impl, _enumeration: &PlainEnum) {}
    fn extend_impl_of_data_enum(
        &self,
        _name: &str,
        _impl_scope: &mut Impl,
        _enumeration: &DataEnum,
    ) {
    }
    fn extend_impl_of_tuple(&self, _name: &str, _impl_scope: &mut Impl, _definition: &RustType) {}
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct RustCodeGenerator {
    models: Vec<Model<Rust>>,
    global_derives: Vec<String>,
    direct_field_access: bool,
    getter_and_setter: bool,
}

impl From<Model<Rust>> for RustCodeGenerator {
    fn from(model: Model<Rust>) -> Self {
        let mut gen = Self::default();
        gen.add_model(model);
        gen
    }
}

impl Default for RustCodeGenerator {
    fn default() -> Self {
        RustCodeGenerator {
            models: Default::default(),
            global_derives: Vec::default(),
            direct_field_access: true,
            getter_and_setter: false,
        }
    }
}

impl Generator<Rust> for RustCodeGenerator {
    type Error = Infallible;

    fn add_model(&mut self, model: Model<Rust>) {
        self.models.push(model);
    }

    fn models(&self) -> &[Model<Rust>] {
        &self.models[..]
    }

    fn models_mut(&mut self) -> &mut [Model<Rust>] {
        &mut self.models[..]
    }

    #[inline]
    fn to_string(&self) -> Result<Vec<(String, String)>, Self::Error> {
        Ok(self.to_string_without_generators())
    }
}

impl RustCodeGenerator {
    pub fn add_global_derive<I: Into<String>>(&mut self, derive: I) {
        self.global_derives.push(derive.into());
    }

    pub fn without_additional_global_derives(mut self) -> Self {
        self.global_derives.clear();
        self
    }

    pub const fn fields_are_pub(&self) -> bool {
        self.direct_field_access
    }

    pub fn set_fields_pub(&mut self, allow: bool) {
        self.direct_field_access = allow;
    }

    pub const fn fields_have_getter_and_setter(&self) -> bool {
        self.getter_and_setter
    }

    pub fn set_fields_have_getter_and_setter(&mut self, allow: bool) {
        self.getter_and_setter = allow;
    }

    pub fn to_string_without_generators(&self) -> Vec<(String, String)> {
        self.to_string_with_generators(&[])
    }

    pub fn to_string_with_generators(
        &self,
        generators: &[&dyn GeneratorSupplement<Rust>],
    ) -> Vec<(String, String)> {
        self.models
            .iter()
            .map(|model| self.model_to_file(model, generators))
            .collect()
    }

    pub fn model_to_file(
        &self,
        model: &Model<Rust>,
        generators: &[&dyn GeneratorSupplement<Rust>],
    ) -> (String, String) {
        let file = {
            let mut string = Self::rust_module_name(&model.name);
            string.push_str(".rs");
            string
        };

        let mut scope = Scope::new();
        generators.iter().for_each(|g| g.add_imports(&mut scope));

        scope.import("asn1rs::prelude", "*");
        for import in &model.imports {
            let from = format!("super::{}", &Self::rust_module_name(&import.from));
            for what in &import.what {
                scope.import(&from, what);
            }
        }

        for vref in &model.value_references {
            scope.raw(&Self::fmt_const(
                &vref.name,
                &vref.role,
                &vref.value.as_rust_const_literal(true),
                0,
            ));
        }

        for definition in &model.definitions {
            self.add_definition(&mut scope, definition);
            Self::impl_definition(&mut scope, definition, generators, self.getter_and_setter);

            generators
                .iter()
                .for_each(|g| g.impl_supplement(&mut scope, definition));
        }

        (file, scope.to_string())
    }

    fn fmt_const(name: &str, r#type: &RustType, value: &impl Display, indent: usize) -> String {
        format!(
            "{}pub const {}: {} = {};",
            "    ".repeat(indent),
            name,
            r#type.to_const_lit_string(),
            if let RustType::Complex(..) = r#type {
                format!("{}::new({})", r#type.to_const_lit_string(), value)
            } else {
                value.to_string()
            }
        )
    }

    pub fn add_definition(&self, scope: &mut Scope, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::Struct {
                fields,
                tag,
                extension_after,
                ordering,
            } => {
                scope.raw(&Self::asn_attribute(
                    match ordering {
                        EncodingOrdering::Keep => "sequence",
                        EncodingOrdering::Sort => "set",
                    },
                    *tag,
                    extension_after.map(|index| fields[index].name().to_string()),
                    &[],
                ));
                Self::add_struct(
                    self.new_struct(scope, name),
                    name,
                    fields,
                    self.direct_field_access,
                )
            }
            Rust::Enum(plain) => {
                scope.raw(&Self::asn_attribute(
                    "enumerated",
                    plain.tag(),
                    plain.extension_after_variant().cloned(),
                    &[],
                ));
                Self::add_enum(
                    self.new_enum(scope, name, true).derive("Default"),
                    name,
                    plain,
                )
            }
            Rust::DataEnum(data) => {
                scope.raw(&Self::asn_attribute(
                    "choice",
                    data.tag(),
                    data.extension_after_variant().map(|v| v.name().to_string()),
                    &[],
                ));
                Self::add_data_enum(self.new_enum(scope, name, false), name, data)
            }
            Rust::TupleStruct {
                r#type,
                tag,
                constants,
            } => {
                scope.raw(&Self::asn_attribute("transparent", *tag, None, &[]));
                Self::add_tuple_struct(
                    self.new_struct(scope, name),
                    name,
                    r#type,
                    self.direct_field_access,
                    None,
                    &constants[..],
                )
            }
        }
    }

    fn add_struct(str_ct: &mut Struct, _name: &str, fields: &[Field], pub_access: bool) {
        for field in fields {
            str_ct.field(
                &format!(
                    "{} {}{}",
                    Self::asn_attribute(
                        Self::asn_attribute_type(&field.r#type().clone().into_asn()),
                        field.tag(),
                        None,
                        field.constants(),
                    ),
                    if pub_access { "pub " } else { "" },
                    Self::rust_field_name(field.name(), true),
                ),
                field.r#type().to_string(),
            );
        }
    }

    fn add_enum(en_m: &mut Enum, _name: &str, rust_enum: &PlainEnum) {
        for (index, variant) in rust_enum.variants().enumerate() {
            let name = Self::rust_variant_name(variant);
            let name = if index == 0 {
                format!("#[default] {name}")
            } else {
                name
            };
            en_m.new_variant(&name);
        }
    }

    fn add_data_enum(en_m: &mut Enum, _name: &str, enumeration: &DataEnum) {
        for variant in enumeration.variants() {
            en_m.new_variant(&format!(
                "{} {}({})",
                Self::asn_attribute(
                    Self::asn_attribute_type(&variant.r#type().clone().into_asn()),
                    variant.tag(),
                    None,
                    &[],
                ),
                Self::rust_variant_name(variant.name()),
                variant.r#type().to_string(),
            ));
        }
    }

    fn add_tuple_struct(
        str_ct: &mut Struct,
        _name: &str,
        inner: &RustType,
        pub_access: bool,
        tag: Option<Tag>,
        constants: &[(String, String)],
    ) {
        str_ct.tuple_field(format!(
            "{} {}{}",
            Self::asn_attribute(
                Self::asn_attribute_type(&inner.clone().into_asn()),
                tag,
                None,
                constants,
            ),
            if pub_access { "pub " } else { "" },
            inner.to_string(),
        ));
    }

    fn asn_attribute<T: ToString>(
        r#type: T,
        tag: Option<Tag>,
        extensible_after: Option<String>,
        constants: &[(String, String)],
    ) -> String {
        format!(
            "#[asn({})]",
            vec![
                Some(r#type.to_string()),
                tag.map(Self::asn_attribute_tag),
                extensible_after.map(Self::asn_attribute_extensible_after),
                if constants.is_empty() {
                    None
                } else {
                    Some(format!(
                        "const({})",
                        constants
                            .iter()
                            .map(|(name, value)| format!("{}({})", name, value))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                }
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ")
        )
    }

    fn asn_attribute_type(r#type: &AsnType) -> String {
        let (name, parameters) = match r#type {
            Type::Boolean => (Cow::Borrowed("boolean"), Vec::default()),
            Type::Integer(integer) => (
                Cow::Borrowed("integer"),
                vec![format!(
                    "{}..{}{}",
                    integer
                        .range
                        .min()
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "min".to_string()),
                    integer
                        .range
                        .max()
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "max".to_string()),
                    if integer.range.extensible() {
                        ",..."
                    } else {
                        ""
                    }
                )],
            ),
            Type::String(size, charset) => (
                Cow::Owned(format!("{:?}string", charset).to_lowercase()),
                vec![size.to_constraint_string()]
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
            Type::OctetString(size) => (
                Cow::Borrowed("octet_string"),
                vec![size.to_constraint_string()]
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
            Type::BitString(bitstring) => (
                Cow::Borrowed("bit_string"),
                vec![vec![bitstring.size.to_constraint_string()]
                    .into_iter()
                    .flatten()
                    .collect()],
            ),
            Type::Null => (Cow::Borrowed("null"), Vec::default()),
            Type::Optional(inner) => (
                Cow::Borrowed("optional"),
                vec![Self::asn_attribute_type(inner)],
            ),
            Type::Default(inner, default) => (
                Cow::Borrowed("default"),
                vec![
                    Self::asn_attribute_type(inner),
                    default.as_rust_const_literal(true).to_string(),
                ],
            ),
            Type::SequenceOf(inner, size) => (
                Cow::Borrowed("sequence_of"),
                vec![
                    size.to_constraint_string(),
                    Some(Self::asn_attribute_type(inner)),
                ]
                .into_iter()
                .flatten()
                .collect(),
            ),
            Type::SetOf(inner, size) => (
                Cow::Borrowed("set_of"),
                vec![
                    size.to_constraint_string(),
                    Some(Self::asn_attribute_type(inner)),
                ]
                .into_iter()
                .flatten()
                .collect(),
            ),

            Type::Sequence(_) => (Cow::Borrowed("sequence"), Vec::default()),
            Type::Set(_) => (Cow::Borrowed("set"), Vec::default()),
            Type::Enumerated(_) => (Cow::Borrowed("enumerated"), Vec::default()),
            Type::Choice(_) => (Cow::Borrowed("choice"), Vec::default()),
            Type::TypeReference(inner, tag) => (
                Cow::Borrowed("complex"),
                vec![Some(inner.clone()), (*tag).map(Self::asn_attribute_tag)]
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
        };
        if parameters.is_empty() {
            name.into_owned()
        } else {
            format!("{}({})", name, parameters.join(", "))
        }
    }

    fn asn_attribute_tag(tag: Tag) -> String {
        match tag {
            Tag::Universal(t) => format!("tag(UNIVERSAL({}))", t),
            Tag::Application(t) => format!("tag(APPLICATION({}))", t),
            Tag::Private(t) => format!("tag(PRIVATE({}))", t),
            Tag::ContextSpecific(t) => format!("tag({})", t),
        }
    }

    fn asn_attribute_extensible_after(variant: String) -> String {
        format!("extensible_after({})", variant)
    }

    fn impl_definition(
        scope: &mut Scope,
        Definition(name, rust): &Definition<Rust>,
        generators: &[&dyn GeneratorSupplement<Rust>],
        getter_and_setter: bool,
    ) {
        match rust {
            Rust::Struct {
                fields,
                tag: _,
                extension_after: _,
                ordering: _,
            } => {
                Self::impl_consts(
                    scope,
                    name,
                    fields
                        .iter()
                        .map(|f| (f.name_type.0.as_str(), &f.name_type.1, &f.constants[..])),
                );
                let implementation = Self::impl_struct(scope, name, fields, getter_and_setter);
                for g in generators {
                    g.extend_impl_of_struct(name, implementation, fields);
                }
            }
            Rust::Enum(r_enum) => {
                let implementation = Self::impl_enum(scope, name, r_enum);
                for g in generators {
                    g.extend_impl_of_enum(name, implementation, r_enum);
                }
            }
            Rust::DataEnum(enumeration) => {
                let implementation = Self::impl_data_enum(scope, name, enumeration);
                for g in generators {
                    g.extend_impl_of_data_enum(name, implementation, enumeration);
                }
                Self::impl_data_enum_default(scope, name, enumeration);
            }
            Rust::TupleStruct {
                r#type: inner,
                tag: _,
                constants,
            } => {
                Self::impl_consts(scope, name, Some(("", inner, &constants[..])).into_iter());
                let implementation = Self::impl_tuple_struct(scope, name, inner);
                for g in generators {
                    g.extend_impl_of_tuple(name, implementation, inner);
                }
                Self::impl_tuple_struct_const_new(scope, name, inner);
                Self::impl_tuple_struct_deref(scope, name, inner);
                Self::impl_tuple_struct_deref_mut(scope, name, inner);
                Self::impl_tuple_struct_from(scope, name, inner);
            }
        }
    }

    fn impl_tuple_struct_const_new(scope: &mut Scope, name: &str, rust: &RustType) {
        scope
            .new_impl(name)
            .new_fn("new")
            .vis("pub const")
            .arg("value", rust.to_string())
            .ret("Self")
            .line("Self(value)");
    }

    fn impl_tuple_struct_deref(scope: &mut Scope, name: &str, rust: &RustType) {
        scope
            .new_impl(name)
            .impl_trait("::core::ops::Deref")
            .associate_type("Target", rust.to_string())
            .new_fn("deref")
            .arg_ref_self()
            .ret(&format!("&{}", rust.to_string()))
            .line("&self.0".to_string());
    }

    fn impl_tuple_struct_deref_mut(scope: &mut Scope, name: &str, rust: &RustType) {
        scope
            .new_impl(name)
            .impl_trait("::core::ops::DerefMut")
            .new_fn("deref_mut")
            .arg_mut_self()
            .ret(&format!("&mut {}", rust.to_string()))
            .line("&mut self.0".to_string());
    }

    fn impl_tuple_struct_from(scope: &mut Scope, name: &str, rust: &RustType) {
        scope
            .new_impl(name)
            .impl_trait(format!("::core::convert::From<{}>", rust.to_string()))
            .new_fn("from")
            .arg("value", &rust.to_string())
            .ret("Self")
            .line("Self(value)");
        scope
            .new_impl(&rust.to_string())
            .impl_trait(format!("::core::convert::From<{}>", name))
            .new_fn("from")
            .arg("value", name)
            .ret("Self")
            .line("value.0");
    }

    fn impl_tuple_struct<'a>(scope: &'a mut Scope, name: &str, rust: &RustType) -> &'a mut Impl {
        let implementation = scope.new_impl(name);
        Self::add_min_max_fn_if_applicable(implementation, None, rust);
        implementation
    }

    fn impl_struct<'a>(
        scope: &'a mut Scope,
        name: &str,
        fields: &[Field],
        getter_and_setter: bool,
    ) -> &'a mut Impl {
        let implementation = scope.new_impl(name);

        for field in fields {
            if getter_and_setter {
                Self::impl_struct_field_get(implementation, field.name(), field.r#type());
                Self::impl_struct_field_get_mut(implementation, field.name(), field.r#type());
                Self::impl_struct_field_set(implementation, field.name(), field.r#type());
            }

            Self::add_min_max_fn_if_applicable(implementation, Some(field.name()), field.r#type());
        }
        implementation
    }

    fn impl_consts<'a>(
        scope: &mut Scope,
        name: &str,
        fields: impl Iterator<Item = (&'a str, &'a RustType, &'a [(String, String)])>,
    ) {
        let mut found_consts = false;
        for (field, r#type, constants) in fields {
            if !found_consts && !constants.is_empty() {
                scope.raw(&format!("impl {} {{", name));
                found_consts = true;
            }
            for (name, value) in constants {
                scope.raw(&Self::fmt_const(
                    &if field.is_empty() {
                        Cow::Borrowed(name)
                    } else {
                        Cow::Owned(format!("{}_{}", field.to_uppercase(), name))
                    },
                    r#type,
                    value,
                    1,
                ));
            }
        }
        if found_consts {
            scope.raw("}");
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

    fn impl_enum<'a>(scope: &'a mut Scope, name: &str, r_enum: &PlainEnum) -> &'a mut Impl {
        let implementation = scope.new_impl(name);

        Self::impl_enum_value_fn(implementation, name, r_enum);
        Self::impl_enum_values_fn(implementation, name, r_enum);
        Self::impl_enum_value_index_fn(implementation, name, r_enum);
        implementation
    }

    fn impl_enum_value_fn(implementation: &mut Impl, name: &str, r_enum: &PlainEnum) {
        let value_fn = implementation
            .new_fn("variant")
            .vis("pub")
            .arg("index", "usize")
            .ret("Option<Self>");

        let mut block_match = Block::new("match index");

        for (index, variant) in r_enum.variants().enumerate() {
            block_match.line(format!(
                "{} => Some({}::{}),",
                index,
                name,
                Self::rust_variant_name(variant)
            ));
        }
        block_match.line("_ => None,");
        value_fn.push_block(block_match);
    }

    fn impl_enum_values_fn(implementation: &mut Impl, name: &str, r_enum: &PlainEnum) {
        let values_fn = implementation
            .new_fn("variants")
            .vis("pub const")
            .ret(format!("[Self; {}]", r_enum.len()))
            .line("[");

        for variant in r_enum.variants() {
            values_fn.line(format!("{}::{},", name, Self::rust_variant_name(variant)));
        }
        values_fn.line("]");
    }

    fn impl_enum_value_index_fn(implementation: &mut Impl, name: &str, r_enum: &PlainEnum) {
        let ordinal_fn = implementation
            .new_fn("value_index")
            .arg_self()
            .vis("pub")
            .ret("usize");

        let mut block = Block::new("match self");
        r_enum
            .variants()
            .enumerate()
            .for_each(|(ordinal, variant)| {
                block.line(format!(
                    "{}::{} => {},",
                    name,
                    Self::rust_variant_name(variant),
                    ordinal
                ));
            });

        ordinal_fn.push_block(block);
    }

    fn impl_data_enum<'a>(
        scope: &'a mut Scope,
        name: &str,
        enumeration: &DataEnum,
    ) -> &'a mut Impl {
        let implementation = scope.new_impl(name);

        Self::impl_data_enum_values_fn(implementation, name, enumeration);
        Self::impl_data_enum_value_index_fn(implementation, name, enumeration);

        for variant in enumeration.variants() {
            let field_name = Self::rust_module_name(variant.name());
            Self::add_min_max_fn_if_applicable(implementation, Some(&field_name), variant.r#type());
        }

        implementation
    }

    fn impl_data_enum_values_fn(implementation: &mut Impl, name: &str, enumeration: &DataEnum) {
        let values_fn = implementation
            .new_fn("variants")
            .vis("pub")
            .ret(format!("[Self; {}]", enumeration.len()))
            .line("[");

        for variant in enumeration.variants() {
            values_fn.line(format!(
                "{}::{}(Default::default()),",
                name,
                Self::rust_variant_name(variant.name())
            ));
        }
        values_fn.line("]");
    }

    fn impl_data_enum_value_index_fn(
        implementation: &mut Impl,
        name: &str,
        enumeration: &DataEnum,
    ) {
        let ordinal_fn = implementation
            .new_fn("value_index")
            .arg_ref_self()
            .vis("pub")
            .ret("usize");

        let mut block = Block::new("match self");
        enumeration
            .variants()
            .enumerate()
            .for_each(|(ordinal, variant)| {
                block.line(format!(
                    "{}::{}(_) => {},",
                    name,
                    Self::rust_variant_name(variant.name()),
                    ordinal
                ));
            });

        ordinal_fn.push_block(block);
    }

    fn impl_data_enum_default(scope: &mut Scope, name: &str, enumeration: &DataEnum) {
        scope
            .new_impl(name)
            .impl_trait("Default")
            .new_fn("default")
            .ret(name as &str)
            .line(format!(
                "{}::{}(Default::default())",
                name,
                Self::rust_variant_name(enumeration.variants().next().unwrap().name())
            ));
    }

    fn add_min_max_fn_if_applicable(
        implementation: &mut Impl,
        field_name: Option<&str>,
        field_type: &RustType,
    ) {
        let prefix = if let Some(field_name) = field_name {
            format!("{}_", field_name)
        } else {
            "value_".to_string()
        };
        if let Some(range) = field_type.integer_range_str() {
            implementation
                .new_fn(&format!("{}min", prefix))
                .vis("pub const")
                .ret(&field_type.to_inner_type_string())
                .line(&Self::format_number_nicely(range.min()));
            implementation
                .new_fn(&format!("{}max", prefix))
                .vis("pub const")
                .ret(&field_type.to_inner_type_string())
                .line(&Self::format_number_nicely(range.max()));
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
        let mut name = name.replace('-', "_");
        if check_for_keywords {
            for keyword in &KEYWORDS {
                if keyword.eq(&name) {
                    name.push('_');
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
            } else if c == '-' || c == '_' {
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

    fn new_struct<'a>(&self, scope: &'a mut Scope, name: &str) -> &'a mut Struct {
        let str_ct = scope
            .new_struct(name)
            .vis("pub")
            .derive("Default")
            .derive("Debug")
            .derive("Clone")
            .derive("PartialEq")
            .derive("Hash");
        self.global_derives.iter().for_each(|derive| {
            str_ct.derive(derive);
        });
        str_ct
    }

    fn new_enum<'a>(&self, scope: &'a mut Scope, name: &str, c_enum: bool) -> &'a mut Enum {
        let en_m = scope
            .new_enum(name)
            .vis("pub")
            .derive("Debug")
            .derive("Clone")
            .derive("PartialEq")
            .derive("Hash");
        if c_enum {
            en_m.derive("Copy").derive("PartialOrd").derive("Eq");
        }
        self.global_derives.iter().for_each(|derive| {
            en_m.derive(derive);
        });
        en_m
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::generator::walker::tests::assert_starts_with_lines;
    use crate::parser::Tokenizer;

    #[test]
    pub fn test_integer_struct_constants() {
        let model = Model::try_from(Tokenizer::default().parse(
            r#"BasicInteger DEFINITIONS AUTOMATIC TAGS ::=
            BEGIN

            MyStruct ::= SEQUENCE {
                item INTEGER { apple(8), banana(9) } (0..255)
            }

            END
        "#,
        ))
        .unwrap()
        .try_resolve()
        .unwrap()
        .to_rust();

        let (_file_name, file_content) = RustCodeGenerator::from(model)
            .without_additional_global_derives()
            .to_string_without_generators()
            .into_iter()
            .next()
            .unwrap();

        assert_starts_with_lines(
            r#"
            use asn1rs::prelude::*;
            
            #[asn(sequence)]
            #[derive(Default, Debug, Clone, PartialEq, Hash)]
            pub struct MyStruct {
                #[asn(integer(0..255), const(APPLE(8), BANANA(9)))] pub item: u8,
            }
            
            impl MyStruct {
                pub const ITEM_APPLE: u8 = 8;
                pub const ITEM_BANANA: u8 = 9;
            }

        "#,
            &file_content,
        );
    }

    #[test]
    pub fn test_integer_tuple_constants() {
        let model = Model::try_from(Tokenizer::default().parse(
            r#"BasicInteger DEFINITIONS AUTOMATIC TAGS ::=
            BEGIN
            
            MyTuple ::= INTEGER { abc(8), bernd(9) } (0..255)
            
            END
        "#,
        ))
        .unwrap()
        .try_resolve()
        .unwrap()
        .to_rust();

        let (_file_name, file_content) = RustCodeGenerator::from(model)
            .without_additional_global_derives()
            .to_string_without_generators()
            .into_iter()
            .next()
            .unwrap();

        assert_starts_with_lines(
            r#"
            use asn1rs::prelude::*;
            
            #[asn(transparent)]
            #[derive(Default, Debug, Clone, PartialEq, Hash)]
            pub struct MyTuple(#[asn(integer(0..255), const(ABC(8), BERND(9)))] pub u8);
            
            impl MyTuple {
                pub const ABC: u8 = 8;
                pub const BERND: u8 = 9;
            }
            
        "#,
            &file_content,
        );
    }
}
