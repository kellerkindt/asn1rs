use crate::gen::RustCodeGenerator;
use crate::model::rust::{rust_module_name, DataEnum, Field, PlainEnum};
use crate::model::{Definition, Model, Range, Rust, RustType, TagProperty};
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
            Rust::Struct {
                fields,
                extension_after: _,
            } => {
                scope.raw(&format!(
                    "type AsnDef{} = {}Sequence<{}>;",
                    name, CRATE_SYN_PREFIX, name
                ));
                for field in fields {
                    self.write_type_declaration(scope, &name, field.name(), field.r#type());
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
                for variant in enm.variants() {
                    self.write_type_declaration(scope, &name, variant.name(), variant.r#type());
                }
            }
            Rust::TupleStruct { r#type: field, .. } => {
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
            RustType::U64(range) => {
                if range
                    .min_max(u64::min_value, || i64::max_value() as u64)
                    .is_some()
                {
                    format!("{}Integer<u64, {}Constraint>", CRATE_SYN_PREFIX, name)
                } else {
                    format!("{}Integer<u64>", CRATE_SYN_PREFIX)
                }
            }
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
        format!("___asn1rs_{}", combined)
    }

    #[must_use]
    pub fn combined_field_type_name(base: &str, name: &str) -> String {
        format!(
            "{}Field{}",
            RustCodeGenerator::rust_variant_name(base),
            RustCodeGenerator::rust_variant_name(name)
        )
    }

    fn write_impl(&self, scope: &mut Scope, Definition(name, r#type): &Definition<Rust>) {
        match r#type {
            Rust::Struct {
                fields,
                extension_after: _,
            } => {
                let constants = fields
                    .iter()
                    .map(|field| {
                        field.constants().iter().map(move |(name, value)| {
                            let field_name = rust_module_name(field.name()).to_uppercase();
                            (
                                field.r#type().clone(),
                                format!("{}_{}", field_name, name),
                                value.clone(),
                            )
                        })
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                if !constants.is_empty() {
                    AsnDefWriter::write_impl_consts(scope, &name, constants);
                }
            }
            Rust::Enum(_) => {}
            Rust::DataEnum(_) => {}
            Rust::TupleStruct { r#type, constants } => {
                let constants = constants
                    .iter()
                    .map(|(name, value)| (r#type.clone(), name.clone(), value.clone()))
                    .collect::<Vec<_>>();
                if !constants.is_empty() {
                    AsnDefWriter::write_impl_consts(scope, &name, constants);
                }
            }
        }
    }

    fn write_impl_consts(
        scope: &mut Scope,
        name: &str,
        constants: Vec<(RustType, String, String)>,
    ) {
        scope.raw(&format!("impl {} {{", name));
        for (r#type, name, value) in constants {
            scope.raw(&format!(
                "    const {}: {} = {};",
                name,
                r#type.to_string(),
                // TODO this does only support a small variety of constant types
                if RustType::String == r#type {
                    format!("\"{}\"", value)
                } else {
                    value
                }
            ));
        }
        scope.raw("}");
    }

    fn write_constraints(&self, scope: &mut Scope, Definition(name, r#type): &Definition<Rust>) {
        match r#type {
            Rust::Struct {
                fields,
                extension_after,
            } => {
                self.write_field_constraints(scope, &name, &fields);
                self.write_sequence_constraint(scope, &name, &fields, *extension_after);
            }
            Rust::Enum(plain) => {
                self.write_enumerated_constraint(scope, &name, plain);
            }
            Rust::DataEnum(data) => self.write_choice_constraint(scope, &name, data),
            Rust::TupleStruct { r#type, constants } => {
                let fields = [Field {
                    name_type: ("0".to_string(), r#type.clone()),
                    tag: None,
                    constants: constants.to_vec(),
                }];
                self.write_field_constraints(scope, &name, &fields[..]);
                self.write_sequence_constraint(scope, &name, &fields[..], None);
            }
        }
    }

    fn write_field_constraints(&self, scope: &mut Scope, name: &str, fields: &[Field]) {
        for field in fields {
            match field.r#type() {
                RustType::Bool => {}
                RustType::I8(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field.name(),
                    &field.r#type().to_string(),
                    &range.wrap_opt(),
                ),
                RustType::U8(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field.name(),
                    &field.r#type().to_string(),
                    &range.wrap_opt(),
                ),
                RustType::I16(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field.name(),
                    &field.r#type().to_string(),
                    &range.wrap_opt(),
                ),
                RustType::U16(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field.name(),
                    &field.r#type().to_string(),
                    &range.wrap_opt(),
                ),
                RustType::I32(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field.name(),
                    &field.r#type().to_string(),
                    &range.wrap_opt(),
                ),
                RustType::U32(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field.name(),
                    &field.r#type().to_string(),
                    &range.wrap_opt(),
                ),
                RustType::I64(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field.name(),
                    &field.r#type().to_string(),
                    &range.wrap_opt(),
                ),
                RustType::U64(range) => Self::write_integer_constraint_type(
                    scope,
                    name,
                    field.name(),
                    &field.r#type().to_string(),
                    range,
                ),
                RustType::String => {}
                RustType::VecU8 => {}
                RustType::Vec(inner) | RustType::Option(inner) => self.write_field_constraints(
                    scope,
                    name,
                    &[Field {
                        name_type: (field.name().to_string(), *inner.clone()),
                        tag: field.tag(),
                        constants: field.constants().to_vec(),
                    }],
                ),
                RustType::Complex(_) => {}
            }
        }
    }

    fn write_sequence_constraint(
        &self,
        scope: &mut Scope,
        name: &str,
        fields: &[Field],
        extension_after_field: Option<usize>,
    ) {
        let mut imp = Impl::new(name);
        imp.impl_trait(format!("{}sequence::Constraint", CRATE_SYN_PREFIX));

        self.write_sequence_constraint_read_fn(&mut imp, name, fields);
        self.write_sequence_constraint_write_fn(&mut imp, name, fields);

        Self::write_sequence_constraint_insert_consts(
            scope,
            name,
            fields,
            extension_after_field,
            imp,
        );
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
                        .extension_after_index()
                        .map(|v| v + 1)
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
                for (index, variant) in choice.variants().enumerate() {
                    match_block.line(format!("Self::{}(_) => {},", variant.name(), index));
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
                for variant in choice.variants() {
                    let combined = Self::combined_field_type_name(name, variant.name());
                    match_block.line(format!(
                        "Self::{}(c) => AsnDef{}::write_value(writer, c),",
                        variant.name(),
                        combined
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
                for (index, variant) in choice.variants().enumerate() {
                    let combined = Self::combined_field_type_name(name, variant.name());
                    match_block.line(format!(
                        "{} => Ok(Some(Self::{}(AsnDef{}::read_value(reader)?))),",
                        index,
                        variant.name(),
                        combined
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
                    choice
                        .extension_after_index()
                        .map(|v| v + 1)
                        .unwrap_or_else(|| choice.len())
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
        range: &Range<Option<T>>,
    ) {
        let combined = Self::combined_field_type_name(name, field) + "Constraint";
        let combined = Self::constraint_impl_name(&combined);

        scope.new_struct(&combined).derive("Default");
        scope.raw(&format!(
            "impl {}numbers::Constraint<{}> for {} {{",
            CRATE_SYN_PREFIX, r#type, combined
        ));
        if let Some(min) = range.min() {
            scope.raw(&format!("const MIN: Option<{}> = Some({});", r#type, min));
        }
        if let Some(max) = range.max() {
            scope.raw(&format!("const MAX: Option<{}> = Some({});", r#type, max));
        }
        scope.raw(&format!("const EXTENSIBLE: bool = {};", range.extensible()));
        scope.raw("}");
    }

    fn write_sequence_constraint_insert_consts(
        scope: &mut Scope,
        name: &str,
        fields: &[Field],
        extension_after_field: Option<usize>,
        imp: Impl,
    ) {
        Self::insert_consts(
            scope,
            imp,
            &[
                format!(
                    "const EXTENDED_AFTER_FIELD: Option<usize> = {:?};",
                    extension_after_field
                ),
                format!("const FIELD_COUNT: usize = {};", fields.len()),
                format!(
                    "const STD_OPTIONAL_FIELDS: usize = {};",
                    fields
                        .iter()
                        .enumerate()
                        .take_while(
                            |(index, _f)| *index <= extension_after_field.unwrap_or(usize::MAX)
                        )
                        .filter(|(_index, f)| f.r#type().is_option())
                        .count()
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

    fn write_sequence_constraint_read_fn(&self, imp: &mut Impl, name: &str, fields: &[Field]) {
        imp.new_fn("read_seq")
            .generic(&format!("R: {}Reader", CRATE_SYN_PREFIX))
            .arg("reader", "&mut R")
            .ret("Result<Self, R::Error>")
            .bound("Self", "Sized")
            .push_block({
                let mut block = Block::new("Ok(Self");

                for field in fields {
                    block.line(format!(
                        "{}: AsnDef{}::read_value(reader)?,",
                        field.name(),
                        Self::combined_field_type_name(name, field.name())
                    ));
                }

                block.after(")");
                block
            });
    }

    fn write_sequence_constraint_write_fn(&self, imp: &mut Impl, name: &str, fields: &[Field]) {
        let body = imp
            .new_fn("write_seq")
            .generic(&format!("W: {}Writer", CRATE_SYN_PREFIX))
            .arg_ref_self()
            .arg("writer", "&mut W")
            .ret("Result<(), W::Error>");

        for field in fields {
            body.line(format!(
                "AsnDef{}::write_value(writer, &self.{})?;",
                Self::combined_field_type_name(name, field.name()),
                field.name(),
            ));
        }

        body.line("Ok(())");
    }

    pub fn stringify(model: &Model<Rust>) -> String {
        let mut scope = Scope::new();
        let myself = Self;

        for definition in &model.definitions {
            myself.write_type_definitions(&mut scope, definition);
            myself.write_impl(&mut scope, definition);
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
    use crate::model::rust::Field;
    use crate::model::{Definition, Model, Rust, RustType};
    use crate::parser::Tokenizer;
    use codegen::Scope;

    fn simple_whatever_sequence() -> Definition<Rust> {
        Definition(
            String::from("Whatever"),
            Rust::struct_from_fields(vec![
                Field::from_name_type("name", RustType::String),
                Field::from_name_type("opt", RustType::Option(Box::new(RustType::String))),
                Field::from_name_type("some", RustType::Option(Box::new(RustType::String))),
            ]),
        )
    }

    fn extensible_potato_sequence() -> Definition<Rust> {
        Definition(
            String::from("Potato"),
            Rust::Struct {
                fields: vec![
                    Field::from_name_type("name", RustType::String),
                    Field::from_name_type("opt", RustType::Option(Box::new(RustType::String))),
                    Field::from_name_type("some", RustType::Option(Box::new(RustType::String))),
                ],
                extension_after: Some(1),
            },
        )
    }

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

        assert_lines(
            r#"
            impl ::asn1rs::syn::sequence::Constraint for Whatever {
                const NAME: &'static str = "Whatever";
                const STD_OPTIONAL_FIELDS: usize = 2;
                const FIELD_COUNT: usize = 3;
                const EXTENDED_AFTER_FIELD: Option<usize> = None;
                
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

    #[test]
    pub fn test_potatoe_struct_has_correct_extensible_constraints() {
        let def = extensible_potato_sequence();
        let mut scope = Scope::new();
        AsnDefWriter.write_constraints(&mut scope, &def);
        let string = scope.to_string();
        println!("{}", string);

        assert_lines(
            r#"
            impl ::asn1rs::syn::sequence::Constraint for Potato {
                const NAME: &'static str = "Potato";
                const STD_OPTIONAL_FIELDS: usize = 1;
                const FIELD_COUNT: usize = 3;
                const EXTENDED_AFTER_FIELD: Option<usize> = Some(1);
                
                fn read_seq<R: ::asn1rs::syn::Reader>(reader: &mut R) -> Result<Self, R::Error>
                where Self: Sized,
                {
                    Ok(Self {
                        name: AsnDefPotatoFieldName::read_value(reader)?,
                        opt: AsnDefPotatoFieldOpt::read_value(reader)?,
                        some: AsnDefPotatoFieldSome::read_value(reader)?,
                    })
                }
                
                fn write_seq<W: ::asn1rs::syn::Writer>(&self, writer: &mut W) -> Result<(), W::Error> {
                    AsnDefPotatoFieldName::write_value(writer, &self.name)?;
                    AsnDefPotatoFieldOpt::write_value(writer, &self.opt)?;
                    AsnDefPotatoFieldSome::write_value(writer, &self.some)?;
                    Ok(())
                }
            }
                
        "#,
            &string,
        );
    }

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
        .to_rust();

        let mut scope = Scope::new();
        AsnDefWriter.write_impl(&mut scope, &model.definitions[0]);
        let string = scope.to_string();
        println!("{}", string);

        assert_lines(
            r#"
            impl MyStruct {
                const ITEM_APPLE: u8 = 8;
                const ITEM_BANANA: u8 = 9;
            }
            
        "#,
            &string,
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
        .to_rust();

        let mut scope = Scope::new();
        AsnDefWriter.write_impl(&mut scope, &model.definitions[0]);
        let string = scope.to_string();
        println!("{}", string);

        assert_lines(
            r#"
            impl MyTuple {
                const ABC: u8 = 8;
                const BERND: u8 = 9;
            }
            
        "#,
            &string,
        );
    }
}
