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
            files.push(Self::model_to_file(
                model,
                &[&UperGenerator, &ProtobufGenerator],
            )?);
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
                                &Self::rust_field_name(&field.name, true),
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
                                .new_fn(&Self::rust_field_name(&field.name, true))
                                .vis("pub")
                                .arg_ref_self()
                                .ret(if field.optional {
                                    format!("&Option<{}>", Self::role_to_type(&field.role))
                                } else {
                                    format!("&{}", Self::role_to_type(&field.role))
                                })
                                .line(format!(
                                    "&self.{}",
                                    Self::rust_field_name(&field.name, true)
                                ));

                            implementation
                                .new_fn(&format!(
                                    "{}_mut",
                                    Self::rust_field_name(&field.name, false)
                                ))
                                .vis("pub")
                                .arg_mut_self()
                                .ret(if field.optional {
                                    format!("&mut Option<{}>", Self::role_to_type(&field.role))
                                } else {
                                    format!("&mut {}", Self::role_to_type(&field.role))
                                })
                                .line(format!(
                                    "&mut self.{}",
                                    Self::rust_field_name(&field.name, true)
                                ));

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
                                        format!("Option<{}>", Self::role_to_type(&field.role))
                                    } else {
                                        Self::role_to_type(&field.role)
                                    },
                                )
                                .line(format!(
                                    "self.{} = value;",
                                    Self::rust_field_name(&field.name, true)
                                ));

                            let min_max = match field.role {
                                Role::Boolean => None,
                                Role::Integer((lower, upper)) => Some((lower, upper)),
                                Role::UnsignedMaxInteger => Some((0, ::std::i64::MAX)),
                                Role::UTF8String => None,
                                Role::Custom(_) => None,
                            };

                            if let Some((min, max)) = min_max {
                                implementation
                                    .new_fn(&format!(
                                        "{}_min",
                                        Self::rust_field_name(&field.name, false)
                                    ))
                                    .vis("pub")
                                    .ret(Self::role_to_type(&field.role))
                                    .line(format!("{}", min));
                                implementation
                                    .new_fn(&format!(
                                        "{}_max",
                                        Self::rust_field_name(&field.name, false)
                                    ))
                                    .vis("pub")
                                    .ret(Self::role_to_type(&field.role))
                                    .line(format!("{}", max));
                            }
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
                    {
                        let implementation = scope.new_impl(&name);
                        {
                            let values_fn = implementation
                                .new_fn("variants")
                                .vis("pub")
                                .ret(format!("[Self; {}]", variants.len()))
                                .line("[");

                            for variant in variants {
                                values_fn.line(format!(
                                    "{}::{},",
                                    name,
                                    Self::rust_variant_name(variant)
                                ));
                            }
                            values_fn.line("]");
                        }
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
            Role::UnsignedMaxInteger => "u64".into(),
            Role::Custom(name) => name.clone(),
            Role::UTF8String => "String".into(),
        };
        type_name
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

pub trait SerializableGenerator {
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
        Generator::new_serializable_impl(scope, impl_for, Self::CODEC)
    }

    fn new_read_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        Generator::new_read_fn(implementation, Self::CODEC)
    }

    fn new_write_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        Generator::new_write_fn(implementation, Self::CODEC)
    }

    fn add_imports(scope: &mut Scope) {
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
                        Role::UnsignedMaxInteger => {
                            block_for.line("writer.write_int_max(*value)?;")
                        }
                        Role::Custom(_custom) => block_for.line("value.write_uper(writer)?;"),
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
                        Role::UnsignedMaxInteger => {
                            block_for.line("me.values.push(reader.read_int_max()?);")
                        }
                        Role::Custom(custom) => block_for
                            .line(format!("me.values.push({}::read_uper(reader)?);", custom)),
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
                                Generator::rust_field_name(&field.name, true),
                            ));
                        }
                    }

                    for field in fields.iter() {
                        let line = match field.role {
                            Role::Boolean => format!(
                                "writer.write_bit({}{})?;",
                                if field.optional { "*" } else { "self." },
                                Generator::rust_field_name(&field.name, true),
                            ),
                            Role::Integer((lower, upper)) => format!(
                                "writer.write_int({}{} as i64, ({} as i64, {} as i64))?;",
                                if field.optional { "*" } else { "self." },
                                Generator::rust_field_name(&field.name, true),
                                lower,
                                upper
                            ),
                            Role::UnsignedMaxInteger => format!(
                                "writer.write_int_max({}{})?;",
                                if field.optional { "*" } else { "self." },
                                Generator::rust_field_name(&field.name, true),
                            ),
                            Role::Custom(ref _type) => format!(
                                "{}{}.write_uper(writer)?;",
                                if field.optional { "" } else { "self." },
                                Generator::rust_field_name(&field.name, true),
                            ),
                            Role::UTF8String => format!(
                                "writer.write_utf8_string(&{}{})?;",
                                if field.optional { "" } else { "self." },
                                Generator::rust_field_name(&field.name, true),
                            ),
                        };
                        if field.optional {
                            let mut b = Block::new(&format!(
                                "if let Some(ref {}) = self.{}",
                                Generator::rust_field_name(&field.name, true),
                                Generator::rust_field_name(&field.name, true),
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
                                Generator::rust_field_name(&field.name, true),
                            ));
                        }
                    }
                    for field in fields.iter() {
                        let line = match field.role {
                            Role::Boolean => format!(
                                "me.{} = {}reader.read_bit()?{};",
                                Generator::rust_field_name(&field.name, true),
                                if field.optional { "Some(" } else { "" },
                                if field.optional { ")" } else { "" },
                            ),
                            Role::Integer((lower, upper)) => format!(
                                "me.{} = {}reader.read_int(({} as i64, {} as i64))? as {}{};",
                                Generator::rust_field_name(&field.name, true),
                                if field.optional { "Some(" } else { "" },
                                lower,
                                upper,
                                Generator::role_to_type(&field.role),
                                if field.optional { ")" } else { "" },
                            ),
                            Role::UnsignedMaxInteger => format!(
                                "me.{} = {}reader.read_int_max()?{};",
                                Generator::rust_field_name(&field.name, true),
                                if field.optional { "Some(" } else { "" },
                                if field.optional { ")" } else { "" },
                            ),
                            Role::Custom(ref _type) => format!(
                                "me.{} = {}{}::read_uper(reader)?{};",
                                Generator::rust_field_name(&field.name, true),
                                if field.optional { "Some(" } else { "" },
                                Generator::role_to_type(&field.role),
                                if field.optional { ")" } else { "" },
                            ),
                            Role::UTF8String => format!(
                                "me.{} = reader.read_utf8_string()?;",
                                Generator::rust_field_name(&field.name, true),
                            ),
                        };
                        if field.optional {
                            let mut block_if = Block::new(&format!(
                                "if {}",
                                Generator::rust_field_name(&field.name, true),
                            ));
                            block_if.line(line);
                            let mut block_else = Block::new("else");
                            block_else.line(format!(
                                "me.{} = None;",
                                Generator::rust_field_name(&field.name, true),
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

pub struct ProtobufGenerator;
impl SerializableGenerator for ProtobufGenerator {
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

impl ProtobufGenerator {
    const CODEC: &'static str = "Protobuf";

    fn new_protobuf_serializable_impl<'a>(scope: &'a mut Scope, impl_for: &str) -> &'a mut Impl {
        Generator::new_serializable_impl(scope, impl_for, Self::CODEC)
    }

    fn new_read_fn<'a>(implementation: &'a mut Impl) -> &'a mut Function {
        Generator::new_read_fn(implementation, Self::CODEC)
    }

    fn new_write_fn<'a, F: FnOnce(&mut Function)>(implementation: &'a mut Impl, once: F) {
        let function = Generator::new_write_fn(implementation, Self::CODEC);
        once(function);
        function.line("Ok(())");
    }

    fn add_imports(scope: &mut Scope) {
        scope.import("asn1c::io::protobuf", Self::CODEC);
        scope.import(
            "asn1c::io::protobuf",
            &format!("Reader as {}Reader", Self::CODEC),
        );
        scope.import(
            "asn1c::io::protobuf",
            &format!("Writer as {}Writer", Self::CODEC),
        );
        scope.import(
            "asn1c::io::protobuf",
            &format!("Error as {}Error", Self::CODEC),
        );
        scope.import(
            "asn1c::io::protobuf",
            &format!("Format as {}Format", Self::CODEC),
        );
    }

    fn generate_serializable_impl(scope: &mut Scope, impl_for: &str, definition: &Definition) {
        use gen::protobuf::Generator as ProtobufGenerator;
        let serializable_implementation = Self::new_protobuf_serializable_impl(scope, impl_for);
        match definition {
            Definition::SequenceOf(_name, aliased) => {
                {
                    Self::new_write_fn(serializable_implementation, |block| {
                        block.line(format!(
                            "writer.write_tag(1, {}Format::LENGTH_DELIMITED);",
                            Self::CODEC
                        ));
                        block.line("let mut bytes = Vec::new();");
                        let mut block_for = Block::new("for value in self.values.iter()");
                        match aliased {
                            Role::Custom(_custom) => {
                                block_for.line(format!(
                                    "value.write_protobuf(&mut bytes as &mut {}Writer)?;",
                                    Self::CODEC
                                ));
                            }
                            r => {
                                block_for.line(format!(
                                    "(&mut bytes as &mut {}Writer).write_{}(*value{})?;",
                                    Self::CODEC,
                                    ProtobufGenerator::role_to_type(r),
                                    Self::role_to_as_statement(r),
                                ));
                            }
                        };
                        block.push_block(block_for);
                        block.line("writer.write_bytes(&bytes[..])?;");
                    });
                }
                {
                    let mut block = Self::new_read_fn(serializable_implementation);
                    block.line("let mut me = Self::default();");
                    block.line("let len = reader.read_varint()? as usize;");
                    let mut block_for = Block::new("for _ in 0..len");
                    match aliased {
                        Role::Custom(custom) => block_for.line(format!(
                            "me.values.push({}::read_protobuf(reader)?);",
                            custom
                        )),
                        r => block_for.line(format!(
                            "me.values.push(reader.read_{}()? as {});",
                            ProtobufGenerator::role_to_type(r),
                            Generator::role_to_type(r),
                        )),
                    };
                    block.push_block(block_for);
                    block.line("Ok(me)");
                }
            }
            Definition::Sequence(_name, fields) => {
                {
                    Self::new_write_fn(serializable_implementation, |block| {
                        for (prev_tag, field) in fields.iter().enumerate() {
                            let block_ : &mut Function = block;
                            let mut block = if field.optional {
                                Block::new(&format!(
                                    "if let Some(ref {}) = self.{}",
                                    Generator::rust_field_name(&field.name, true),
                                    Generator::rust_field_name(&field.name, true),
                                ))
                            } else {
                                Block::new("")
                            };

                            match &field.role {
                                Role::Custom(_custom) => {
                                    block.line(format!(
                                        "writer.write_tag({}, {}Format::LENGTH_DELIMITED);",
                                        prev_tag + 1,
                                        Self::CODEC
                                    ));
                                    block.line("let mut vec = Vec::new();");
                                    block.line(format!(
                                        "{}{}.write_protobuf(&mut vec as &mut {}Writer)?;",
                                        if field.optional { "" } else { "self." },
                                        Generator::rust_field_name(&field.name, true),
                                        Self::CODEC,
                                    ));
                                    block.line("writer.write_bytes(&vec[..])?;");
                                }
                                r => {
                                    block.line(format!(
                                        "writer.write_tagged_{}({}, {}{}{})?;",
                                        ProtobufGenerator::role_to_type(r),
                                        prev_tag + 1,
                                        if "string".eq_ignore_ascii_case(
                                            &ProtobufGenerator::role_to_type(r)
                                        ) {
                                            if field.optional {
                                                ""
                                            } else {
                                                "&self."
                                            }
                                        } else {
                                            if field.optional {
                                                "*"
                                            } else {
                                                "self."
                                            }
                                        },
                                        Generator::rust_field_name(&field.name, true),
                                        Self::role_to_as_statement(r),
                                    ));
                                }
                            };
                            block_.push_block(block);
                        }
                    });
                }
                {
                    let block = Self::new_read_fn(serializable_implementation);
                    for field in fields.iter() {
                        block.line(format!(
                            "let mut read_{} = None;",
                            Generator::rust_field_name(&field.name, false)
                        ));
                    }

                    let mut block_reader_loop = Block::new("while let Ok(tag) = reader.read_tag()");
                    let mut block_match_tag = Block::new("match tag.0");

                    for (prev_tag, field) in fields.iter().enumerate() {
                        block_match_tag.line(format!(
                            "{} => read_{} = Some({}),",
                            prev_tag + 1,
                            Generator::rust_field_name(&field.name, false),
                            match &field.role {
                                Role::Custom(name) => format!("{}::read_protobuf(reader)?", name),
                                role => format!(
                                    "reader.read_{}()?",
                                    ProtobufGenerator::role_to_type(role)
                                ),
                            },
                        ));
                    }

                    block_match_tag.line(format!(
                        "_ => return Err({}Error::InvalidTagReceived(tag.0)),",
                        Self::CODEC
                    ));
                    block_reader_loop.push_block(block_match_tag);
                    block.push_block(block_reader_loop);
                    let mut return_block = Block::new(&format!("Ok({}", _name));
                    for field in fields.iter() {
                        return_block.line(&format!(
                            "{}: read_{}.map(|v| v as {}){},",
                            Generator::rust_field_name(&field.name, true),
                            Generator::rust_field_name(&field.name, false),
                            Generator::role_to_type(&field.role),
                            if field.optional {
                                "".into()
                            } else {
                                format!(
                                    ".ok_or({}Error::MissingRequiredField(\"{}::{}\"))?",
                                    Self::CODEC,
                                    _name,
                                    Generator::rust_field_name(&field.name, true)
                                )
                            },
                        ));
                    }

                    return_block.after(")");
                    block.push_block(return_block);
                }
            }
            Definition::Enumerated(name, variants) => {
                {
                    Self::new_write_fn(serializable_implementation, |block| {
                        let mut outer_block = Block::new("match self");
                        for (prev_tag, variant) in variants.iter().enumerate() {
                            outer_block.line(format!(
                                "{}::{} => writer.write_varint({})?,",
                                name,
                                Generator::rust_variant_name(&variant),
                                prev_tag + 1,
                            ));
                        }
                        block.push_block(outer_block);
                    });
                }
                {
                    let mut block = Self::new_read_fn(serializable_implementation);
                    block.line(format!("let tag = reader.read_tag()?;",));
                    let mut block_match = Block::new("match tag.0");
                    for (prev_tag, variant) in variants.iter().enumerate() {
                        block_match.line(format!(
                            "{} => Ok({}::{}),",
                            prev_tag + 1,
                            name,
                            Generator::rust_variant_name(&variant),
                        ));
                    }
                    block_match.line(format!(
                        "_ => Err({}Error::InvalidTagReceived(tag.0))",
                        Self::CODEC,
                    ));
                    block.push_block(block_match);
                }
            }
        }
    }

    fn role_to_as_statement(role: &Role) -> String {
        use gen::protobuf::Generator as ProtobufGenerator;
        match ProtobufGenerator::role_to_type(role).as_str() {
            "sfixed32" => " as i32",
            "sfixed64" => " as i64",
            "uint64" => " as u64",
            _ => "",
        }.into()
    }
}
