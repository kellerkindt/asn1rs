use crate::gen::RustCodeGenerator;
use crate::model::rust::{DataEnum, PlainEnum};
use crate::model::{Definition, Model, Range, Rust, RustType};
use codegen::{Block, Impl, Scope};
use std::fmt::Display;

pub const CRATE_SYN_PREFIX: &str = "::asn1rs::syn::";

pub struct AsnDefWriter;

impl AsnDefWriter {
    fn write_type_definitions(
        &self,
        scope: &mut Scope,
        Definition(name, r#type): &Definition<Rust>,
    ) {
        match r#type {
            Rust::Struct(fields) => {
                scope.raw(&format!(
                    "type AsnDef{} = {}Sequence<{}>;",
                    name, CRATE_SYN_PREFIX, name
                ));
                for (field, r#type) in fields {
                    self.write_type_declaration(scope, &name, &field, r#type);
                }
            }
            Rust::Enum(_enm) => {
                scope.raw(&format!(
                    "type AsnDef{} = {}Enumerated<{}>;",
                    name, CRATE_SYN_PREFIX, name
                ));
            }
            Rust::DataEnum(enm) => {
                scope.raw(&format!(
                    "type AsnDef{} = {}Choice<{}>;",
                    name, CRATE_SYN_PREFIX, name
                ));
                for (field, r#type) in enm.variants() {
                    self.write_type_declaration(scope, &name, &field, r#type);
                }
            }
            Rust::TupleStruct(field) => {
                scope.raw(&format!(
                    "type AsnDef{} = {}Sequence<{}>;",
                    name, CRATE_SYN_PREFIX, name
                ));
                self.write_type_declaration(scope, &name, "0", field);
            }
        }
    }

    #[must_use]
    pub fn type_declaration(r#type: &RustType, name: &str) -> String {
        match r#type {
            RustType::Bool => format!("{}Boolean", CRATE_SYN_PREFIX),
            RustType::I8(_) => format!("{}Integer<i8, {}Constraint>", CRATE_SYN_PREFIX, name),
            RustType::U8(_) => format!("{}Integer<u8, {}Constraint>", CRATE_SYN_PREFIX, name),
            RustType::I16(_) => format!("{}Integer<i16, {}Constraint>", CRATE_SYN_PREFIX, name),
            RustType::U16(_) => format!("{}Integer<u16, {}Constraint>", CRATE_SYN_PREFIX, name),
            RustType::I32(_) => format!("{}Integer<i32, {}Constraint>", CRATE_SYN_PREFIX, name),
            RustType::U32(_) => format!("{}Integer<u32, {}Constraint>", CRATE_SYN_PREFIX, name),
            RustType::I64(_) => format!("{}Integer<i64, {}Constraint>", CRATE_SYN_PREFIX, name),
            RustType::U64(Some(_)) => {
                format!("{}Integer<u64, {}Constraint>", CRATE_SYN_PREFIX, name)
            }
            RustType::U64(None) => format!("{}Integer<u64>", CRATE_SYN_PREFIX),
            RustType::String => format!("{}Utf8String", CRATE_SYN_PREFIX),
            RustType::VecU8 => format!("{}OctetString", CRATE_SYN_PREFIX),
            RustType::Vec(inner) => format!(
                "{}SequenceOf<{}>",
                CRATE_SYN_PREFIX,
                Self::type_declaration(&*inner, name)
            ),
            RustType::Option(inner) => format!("Option<{}>", Self::type_declaration(&*inner, name)),
            RustType::Complex(inner) => format!("{}Complex<{}>", CRATE_SYN_PREFIX, inner),
        }
    }

    fn write_type_declaration(&self, scope: &mut Scope, base: &str, name: &str, r#type: &RustType) {
        let combined = Self::combined_field_type_name(base, name);
        let type_dec = Self::type_declaration(r#type, &Self::constraint_impl_name(&combined));
        scope.raw(&format!("type AsnDef{} = {};", combined, type_dec));
    }

    fn constraint_impl_name(combined: &str) -> String {
        format!("___ans1rs_{}", combined)
    }

    #[must_use]
    pub fn combined_field_type_name(base: &str, name: &str) -> String {
        format!(
            "{}Field{}",
            RustCodeGenerator::rust_variant_name(base),
            RustCodeGenerator::rust_variant_name(name)
        )
    }

    fn write_constraints(&self, scope: &mut Scope, Definition(name, r#type): &Definition<Rust>) {
        match r#type {
            Rust::Struct(fields) => {
                self.write_field_constraints(scope, &name, &fields);
                self.write_sequence_constraint(scope, &name, &fields);
            }
            Rust::Enum(plain) => {
                self.write_enumerated_constraint(scope, &name, plain);
            }
            Rust::DataEnum(data) => self.write_choice_constraint(scope, &name, data),
            Rust::TupleStruct(field) => {
                let fields = [(String::from("0"), field.clone())];
                self.write_field_constraints(scope, &name, &fields[..]);
                self.write_sequence_constraint(scope, &name, &fields[..]);
            }
        }
    }

    fn write_field_constraints(
        &self,
        scope: &mut Scope,
        name: &str,
        fields: &[(String, RustType)],
    ) {
        for (field, r#type) in fields {
            match r#type {
                RustType::Bool => {}
                RustType::I8(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field,
                    &r#type.to_string(),
                    range,
                ),
                RustType::U8(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field,
                    &r#type.to_string(),
                    range,
                ),
                RustType::I16(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field,
                    &r#type.to_string(),
                    range,
                ),
                RustType::U16(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field,
                    &r#type.to_string(),
                    range,
                ),
                RustType::I32(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field,
                    &r#type.to_string(),
                    range,
                ),
                RustType::U32(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field,
                    &r#type.to_string(),
                    range,
                ),
                RustType::I64(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field,
                    &r#type.to_string(),
                    range,
                ),
                RustType::U64(Some(range)) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field,
                    &r#type.to_string(),
                    range,
                ),
                RustType::U64(_) => {}
                RustType::String => {}
                RustType::VecU8 => {}
                RustType::Vec(inner) => self.write_field_constraints(
                    scope,
                    name,
                    &[(field.to_string(), *inner.clone())],
                ),
                RustType::Option(inner) => self.write_field_constraints(
                    scope,
                    name,
                    &[(field.to_string(), *inner.clone())],
                ),
                RustType::Complex(_) => {}
            }
        }
    }

    fn write_sequence_constraint(
        &self,
        scope: &mut Scope,
        name: &str,
        fields: &[(String, RustType)],
    ) {
        let mut imp = Impl::new(name);
        imp.impl_trait(format!("{}sequence::Constraint", CRATE_SYN_PREFIX));

        self.write_sequence_constraint_read_fn(&mut imp, name, fields);
        self.write_sequence_constraint_write_fn(&mut imp, name, fields);

        Self::write_sequence_constraint_insert_consts(scope, name, fields, imp);
    }

    fn impl_readable(&self, scope: &mut Scope, name: &str) {
        let imp = scope
            .new_impl(name)
            .impl_trait(format!("{}Readable", CRATE_SYN_PREFIX));

        imp.new_fn("read")
            .generic(&format!("R: {}Reader", CRATE_SYN_PREFIX))
            .arg("reader", "&mut R")
            .ret("Result<Self, R::Error>")
            .line(format!("AsnDef{}::read_value(reader)", name));
    }

    fn impl_writable(&self, scope: &mut Scope, name: &str) {
        let imp = scope
            .new_impl(name)
            .impl_trait(format!("{}Writable", CRATE_SYN_PREFIX));

        imp.new_fn("write")
            .generic(&format!("W: {}Writer", CRATE_SYN_PREFIX))
            .arg_ref_self()
            .arg("writer", "&mut W")
            .ret("Result<(), W::Error>")
            .line(format!("AsnDef{}::write_value(writer, self)", name));
    }

    fn write_enumerated_constraint(&self, scope: &mut Scope, name: &str, enumerated: &PlainEnum) {
        let mut imp = Impl::new(name);
        imp.impl_trait(format!("{}enumerated::Constraint", CRATE_SYN_PREFIX));

        imp.new_fn("to_choice_index")
            .arg_ref_self()
            .ret("usize")
            .push_block({
                let mut match_block = Block::new("match self");
                for (index, variant) in enumerated.variants().enumerate() {
                    match_block.line(format!("Self::{} => {},", variant, index));
                }
                match_block
            });

        imp.new_fn("from_choice_index")
            .arg("index", "usize")
            .ret("Option<Self>")
            .push_block({
                let mut match_block = Block::new("match index");
                for (index, variant) in enumerated.variants().enumerate() {
                    match_block.line(format!("{} => Some(Self::{}),", index, variant));
                }
                match_block.line("_ => None,");
                match_block
            });

        Self::insert_consts(
            scope,
            imp,
            &[
                format!("const NAME: &'static str = \"{}\";", name),
                format!("const VARIANT_COUNT: usize = {};", enumerated.len()),
                format!(
                    "const STD_VARIANT_COUNT: usize = {};",
                    enumerated
                        .last_standard_index()
                        .unwrap_or_else(|| enumerated.len())
                ),
                format!("const EXTENSIBLE: bool = {};", enumerated.is_extensible()),
            ],
        );
    }

    fn write_choice_constraint(&self, scope: &mut Scope, name: &str, choice: &DataEnum) {
        let mut imp = Impl::new(name);
        imp.impl_trait(format!("{}choice::Constraint", CRATE_SYN_PREFIX));

        imp.new_fn("to_choice_index")
            .arg_ref_self()
            .ret("usize")
            .push_block({
                let mut match_block = Block::new("match self");
                for (index, (variant, _type)) in choice.variants().enumerate() {
                    match_block.line(format!("Self::{}(_) => {},", variant, index));
                }
                match_block
            });

        imp.new_fn("write_content")
            .generic(&format!("W: {}Writer", CRATE_SYN_PREFIX))
            .arg_ref_self()
            .arg("writer", "&mut W")
            .ret("Result<(), W::Error>")
            .push_block({
                let mut match_block = Block::new("match self");
                for (variant, _type) in choice.variants() {
                    let combined = Self::combined_field_type_name(name, variant);
                    match_block.line(format!(
                        "Self::{}(c) => AsnDef{}::write_value(writer, c),",
                        variant, combined
                    ));
                }
                match_block
            });

        imp.new_fn("read_content")
            .generic(&format!("R: {}Reader", CRATE_SYN_PREFIX))
            .arg("index", "usize")
            .arg("reader", "&mut R")
            .ret("Result<Option<Self>, R::Error>")
            .push_block({
                let mut match_block = Block::new("match index");
                for (index, (variant, _type)) in choice.variants().enumerate() {
                    let combined = Self::combined_field_type_name(name, variant);
                    match_block.line(format!(
                        "{} => Ok(Some(Self::{}(AsnDef{}::read_value(reader)?))),",
                        index, variant, combined
                    ));
                }
                match_block.line("_ => Ok(None),");
                match_block
            });

        Self::insert_consts(
            scope,
            imp,
            &[
                format!("const NAME: &'static str = \"{}\";", name),
                format!("const VARIANT_COUNT: usize = {};", choice.len()),
                format!(
                    "const STD_VARIANT_COUNT: usize = {};",
                    choice.last_standard_index().unwrap_or_else(|| choice.len())
                ),
                format!("const EXTENSIBLE: bool = {};", choice.is_extensible()),
            ],
        );
    }

    fn write_integer_constraint_type<T: Display>(
        scope: &mut Scope,
        name: &str,
        field: &str,
        r#type: &str,
        Range(min, max): &Range<T>,
    ) {
        let combined = Self::combined_field_type_name(name, field) + "Constraint";
        let combined = Self::constraint_impl_name(&combined);

        scope.new_struct(&combined).derive("Default");
        scope.raw(&format!(
            "impl {}numbers::Constraint<{}> for {} {{",
            CRATE_SYN_PREFIX, r#type, combined
        ));
        scope.raw(&format!("const MIN: Option<{}> = Some({});", r#type, min));
        scope.raw(&format!("const MAX: Option<{}> = Some({});", r#type, max));
        scope.raw("}");
    }

    fn write_sequence_constraint_insert_consts(
        scope: &mut Scope,
        name: &str,
        fields: &[(String, RustType)],
        imp: Impl,
    ) {
        Self::insert_consts(
            scope,
            imp,
            &[
                format!(
                    "const OPTIONAL_FIELDS: usize = {};",
                    fields.iter().filter(|f| f.1.is_option()).count()
                ),
                format!("const NAME: &'static str = \"{}\";", name),
            ],
        );
    }

    fn insert_consts<S: ToString, I: IntoIterator<Item = S>>(
        scope: &mut Scope,
        imp: Impl,
        consts: I,
    ) {
        let string = Scope::new().push_impl(imp).to_string();
        let mut lines = string.lines().map(ToString::to_string).collect::<Vec<_>>();

        for cnst in consts {
            lines.insert(1, cnst.to_string());
        }

        scope.raw(&lines.join("\n"));
    }

    fn write_sequence_constraint_read_fn(
        &self,
        imp: &mut Impl,
        name: &str,
        fields: &[(String, RustType)],
    ) {
        imp.new_fn("read_seq")
            .generic(&format!("R: {}Reader", CRATE_SYN_PREFIX))
            .arg("reader", "&mut R")
            .ret("Result<Self, R::Error>")
            .bound("Self", "Sized")
            .push_block({
                let mut block = Block::new("Ok(Self");

                for (field, _type) in fields {
                    block.line(format!(
                        "{}: AsnDef{}::read_value(reader)?,",
                        field,
                        Self::combined_field_type_name(name, field)
                    ));
                }

                block.after(")");
                block
            });
    }

    fn write_sequence_constraint_write_fn(
        &self,
        imp: &mut Impl,
        name: &str,
        fields: &[(String, RustType)],
    ) {
        let body = imp
            .new_fn("write_seq")
            .generic(&format!("W: {}Writer", CRATE_SYN_PREFIX))
            .arg_ref_self()
            .arg("writer", "&mut W")
            .ret("Result<(), W::Error>");

        for (field, _type) in fields {
            body.line(format!(
                "AsnDef{}::write_value(writer, &self.{})?;",
                Self::combined_field_type_name(name, field),
                field,
            ));
        }

        body.line("Ok(())");
    }

    pub fn stringify(model: &Model<Rust>) -> String {
        let mut scope = Scope::new();
        let myself = Self;

        for definition in &model.definitions {
            myself.write_type_definitions(&mut scope, definition);
            myself.write_constraints(&mut scope, definition);
            myself.impl_readable(&mut scope, &definition.0);
            myself.impl_writable(&mut scope, &definition.0);
        }

        scope.to_string()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::gen::rust::walker::AsnDefWriter;
    use crate::model::{Definition, Rust, RustType};
    use codegen::Scope;

    fn simple_whatever_sequence() -> Definition<Rust> {
        Definition(
            String::from("Whatever"),
            Rust::Struct(vec![
                (String::from("name"), RustType::String),
                (
                    String::from("opt"),
                    RustType::Option(Box::new(RustType::String)),
                ),
                (
                    String::from("some"),
                    RustType::Option(Box::new(RustType::String)),
                ),
            ]),
        )
    }

    #[test]
    pub fn test_whatever_struct_type_declaration() {
        let def = simple_whatever_sequence();
        let mut scope = Scope::new();
        AsnDefWriter.write_type_definitions(&mut scope, &def);
        let string = scope.to_string();
        println!("{}", string);
        let mut lines = string.lines().filter(|l| !l.is_empty());
        assert_eq!(
            Some("type AsnDefWhatever = ::asn1rs::syn::Sequence<Whatever>;"),
            lines.next()
        );
        assert_eq!(
            Some("type AsnDefWhateverFieldName = ::asn1rs::syn::Utf8String;"),
            lines.next()
        );
        assert_eq!(
            Some("type AsnDefWhateverFieldOpt = Option<::asn1rs::syn::Utf8String>;"),
            lines.next()
        );
        assert_eq!(
            Some("type AsnDefWhateverFieldSome = Option<::asn1rs::syn::Utf8String>;"),
            lines.next()
        );
    }

    #[test]
    pub fn test_whatever_struct_constraint_and_read_write_impl() {
        let def = simple_whatever_sequence();
        let mut scope = Scope::new();
        AsnDefWriter.write_constraints(&mut scope, &def);
        AsnDefWriter.impl_readable(&mut scope, &def.0);
        AsnDefWriter.impl_writable(&mut scope, &def.0);
        let string = scope.to_string();
        println!("{}", string);

        fn assert_lines(expected: &str, actual: &str) {
            let mut expected = expected.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
            let mut actual = actual.lines().map(|l| l.trim()).filter(|l| !l.is_empty());

            loop {
                let expected = expected.next();
                let actual = actual.next();
                assert_eq!(expected, actual);
                if expected.is_none() && actual.is_none() {
                    break;
                }
            }
        }

        assert_lines(
            r#"
            impl ::asn1rs::syn::sequence::Constraint for Whatever {
                const NAME: &'static str = "Whatever";
                const OPTIONAL_FIELDS: usize = 2;
                
                fn read_seq<R: ::asn1rs::syn::Reader>(reader: &mut R) -> Result<Self, R::Error>
                where Self: Sized,
                {
                    Ok(Self {
                        name: AsnDefWhateverFieldName::read_value(reader)?,
                        opt: AsnDefWhateverFieldOpt::read_value(reader)?,
                        some: AsnDefWhateverFieldSome::read_value(reader)?,
                    })
                }
                
                fn write_seq<W: ::asn1rs::syn::Writer>(&self, writer: &mut W) -> Result<(), W::Error> {
                    AsnDefWhateverFieldName::write_value(writer, &self.name)?;
                    AsnDefWhateverFieldOpt::write_value(writer, &self.opt)?;
                    AsnDefWhateverFieldSome::write_value(writer, &self.some)?;
                    Ok(())
                }
            }
            
            impl ::asn1rs::syn::Readable for Whatever {
                fn read<R: ::asn1rs::syn::Reader>(reader: &mut R) -> Result<Self, R::Error> {
                    AsnDefWhatever::read_value(reader)
                }
            }
            
            impl ::asn1rs::syn::Writable for Whatever {
                fn write<W: ::asn1rs::syn::Writer>(&self, writer: &mut W) -> Result<(), W::Error> {
                    AsnDefWhatever::write_value(writer, self)
                }
            }
                
        "#,
            &string,
        );
    }
}
