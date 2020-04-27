use crate::gen::rust::GeneratorSupplement;
use crate::gen::RustCodeGenerator;
use crate::model::{Definition, Model, Range, Rust, RustType};
use codegen::{Block, Impl, Scope};
use std::fmt::Display;

pub const CRATE_SYN_PREFIX: &str = "::asn1rs::syn::";

pub struct AsnDefWalker;

impl AsnDefWalker {
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
            Rust::Enum(_) => {}
            Rust::DataEnum(_) => {}
            Rust::TupleStruct(_) => {}
        }
    }

    #[must_use]
    pub fn type_declaration(r#type: &RustType, name: &str) -> String {
        match r#type {
            RustType::Bool => format!("{}Bool", CRATE_SYN_PREFIX),
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
            RustType::U64(None) => format!("{}Integer", CRATE_SYN_PREFIX),
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
        let type_dec = Self::type_declaration(r#type, &combined);
        scope.raw(&format!("type AsnDef{} = {};", combined, type_dec));
    }

    #[must_use]
    pub fn combined_field_type_name(base: &str, name: &str) -> String {
        format!(
            "{}{}",
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
            Rust::Enum(_) => {}
            Rust::DataEnum(_) => {}
            Rust::TupleStruct(_) => {}
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
                RustType::Vec(_) => {}
                RustType::Option(_) => {}
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

        scope.raw(&Self::write_sequence_constraint_insert_consts(
            name, fields, imp,
        ));
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

    fn write_integer_constraint_type<T: Display>(
        scope: &mut Scope,
        name: &str,
        field: &str,
        r#type: &str,
        Range(min, max): &Range<T>,
    ) {
        let combined = Self::combined_field_type_name(name, field) + "Constraint";

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
        name: &str,
        fields: &[(String, RustType)],
        imp: Impl,
    ) -> String {
        let string = Scope::new().push_impl(imp).to_string();
        let mut lines = string.lines().map(ToString::to_string).collect::<Vec<_>>();
        lines.insert(
            1,
            format!(
                "    const OPTIONAL_FIELDS: usize = {};",
                fields.iter().filter(|f| f.1.is_option()).count()
            ),
        );
        lines.insert(1, format!("const NAME: &'static str = \"{}\";", name));
        lines.join("\n")
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

        myself.add_imports(&mut scope);

        for definition in &model.definitions {
            myself.impl_supplement(&mut scope, definition);
        }

        scope.to_string()
    }
}

impl GeneratorSupplement<Rust> for AsnDefWalker {
    fn add_imports(&self, scope: &mut Scope) {
        scope.raw("use asn1rs::prelude::*;");
    }

    fn impl_supplement(&self, scope: &mut Scope, definition: &Definition<Rust>) {
        self.write_type_definitions(scope, definition);
        self.write_constraints(scope, definition);
        self.impl_readable(scope, &definition.0);
        self.impl_writable(scope, &definition.0);
    }
}

#[cfg(test)]
pub mod tests {
    use crate::gen::rust::walker::AsnDefWalker;
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
        AsnDefWalker.write_type_definitions(&mut scope, &def);
        let string = scope.to_string();
        println!("{}", string);
        let mut lines = string.lines().filter(|l| !l.is_empty());
        assert_eq!(
            Some("type AsnDefWhatever = ::asn1rs::syn::Sequence<Whatever>;"),
            lines.next()
        );
        assert_eq!(
            Some("type AsnDefWhateverName = ::asn1rs::syn::Utf8String;"),
            lines.next()
        );
        assert_eq!(
            Some("type AsnDefWhateverOpt = Option<::asn1rs::syn::Utf8String>;"),
            lines.next()
        );
        assert_eq!(
            Some("type AsnDefWhateverSome = Option<::asn1rs::syn::Utf8String>;"),
            lines.next()
        );
    }

    #[test]
    pub fn test_whatever_struct_constraint_and_read_write_impl() {
        let def = simple_whatever_sequence();
        let mut scope = Scope::new();
        AsnDefWalker.write_constraints(&mut scope, &def);
        AsnDefWalker.impl_readable(&mut scope, &def.0);
        AsnDefWalker.impl_writable(&mut scope, &def.0);
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
                        name: AsnDefWhateverName::read_value(reader)?,
                        opt: AsnDefWhateverOpt::read_value(reader)?,
                        some: AsnDefWhateverSome::read_value(reader)?,
                    })
                }
                
                fn write_seq<W: ::asn1rs::syn::Writer>(&self, writer: &mut W) -> Result<(), W::Error> {
                    AsnDefWhateverName::write_value(writer, &self.name)?;
                    AsnDefWhateverOpt::write_value(writer, &self.opt)?;
                    AsnDefWhateverSome::write_value(writer, &self.some)?;
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
