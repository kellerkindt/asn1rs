use crate::gen::Generator;
use crate::model::Definition;
use crate::model::Model;
use crate::model::Protobuf;
use crate::model::ProtobufType;
use std::fmt::Error as FmtError;
use std::fmt::Write;

#[derive(Debug)]
pub enum Error {
    Fmt(FmtError),
}

impl From<FmtError> for Error {
    fn from(e: FmtError) -> Self {
        Error::Fmt(e)
    }
}

#[derive(Debug, Default)]
pub struct ProtobufDefGenerator {
    models: Vec<Model<Protobuf>>,
}

impl Generator<Protobuf> for ProtobufDefGenerator {
    type Error = Error;

    fn add_model(&mut self, model: Model<Protobuf>) {
        self.models.push(model);
    }

    fn models(&self) -> &[Model<Protobuf>] {
        &self.models[..]
    }

    fn models_mut(&mut self) -> &mut [Model<Protobuf>] {
        &mut self.models[..]
    }

    fn to_string(&self) -> Result<Vec<(String, String)>, <Self as Generator<Protobuf>>::Error> {
        let mut files = Vec::new();
        for model in &self.models {
            files.push(Self::generate_file(model)?);
        }
        Ok(files)
    }
}

impl ProtobufDefGenerator {
    pub fn generate_file(model: &Model<Protobuf>) -> Result<(String, String), Error> {
        let file_name = Self::model_file_name(&model.name);
        let mut content = String::new();
        Self::append_header(&mut content, model)?;
        Self::append_imports(&mut content, model)?;
        for definition in &model.definitions {
            Self::append_definition(&mut content, model, definition)?;
        }
        Ok((file_name, content))
    }

    pub fn append_header(target: &mut Write, model: &Model<Protobuf>) -> Result<(), Error> {
        writeln!(target, "syntax = 'proto3';")?;
        writeln!(target, "package {};", Self::model_to_package(&model.name))?;
        writeln!(target)?;
        Ok(())
    }

    pub fn append_imports(target: &mut Write, model: &Model<Protobuf>) -> Result<(), Error> {
        for import in &model.imports {
            writeln!(target, "import '{}';", Self::model_file_name(&import.from))?;
        }
        writeln!(target)?;
        Ok(())
    }

    pub fn append_definition(
        target: &mut Write,
        model: &Model<Protobuf>,
        Definition(name, protobuf): &Definition<Protobuf>,
    ) -> Result<(), Error> {
        match protobuf {
            Protobuf::Enum(variants) => {
                writeln!(target, "enum {} {{", name)?;
                for (tag, variant) in variants.iter().enumerate() {
                    Self::append_variant(target, &variant, tag)?;
                }
                writeln!(target, "}}")?;
            }
            Protobuf::Message(fields) => {
                writeln!(target, "message {} {{", name)?;
                for (prev_tag, (field_name, field_type)) in fields.iter().enumerate() {
                    Self::append_field(target, model, &field_name, &field_type, prev_tag + 1)?;
                }
                writeln!(target, "}}")?;
            }
        }
        Ok(())
    }

    pub fn append_field(
        target: &mut Write,
        model: &Model<Protobuf>,
        name: &str,
        role: &ProtobufType,
        tag: usize,
    ) -> Result<(), Error> {
        writeln!(
            target,
            "    {} {}{};",
            Self::role_to_full_type(role, model),
            Self::field_name(name),
            if let ProtobufType::OneOf(variants) = role {
                let mut inner = String::new();
                writeln!(&mut inner, " {{")?;
                for (index, (variant_name, variant_type)) in variants.iter().enumerate() {
                    writeln!(
                        &mut inner,
                        "      {} {} = {};",
                        Self::role_to_full_type(variant_type, model),
                        variant_name,
                        index + 1
                    )?;
                }
                write!(&mut inner, "    }}")?;
                inner
            } else {
                format!(" = {}", tag)
            }
        )?;
        Ok(())
    }

    pub fn append_variant(target: &mut Write, variant: &str, tag: usize) -> Result<(), Error> {
        writeln!(target, "    {} = {};", Self::variant_name(&variant), tag)?;
        Ok(())
    }

    pub fn role_to_full_type(role: &ProtobufType, model: &Model<Protobuf>) -> String {
        match role {
            ProtobufType::Complex(name) => {
                let mut prefixed = String::new();
                'outer: for import in &model.imports {
                    for what in &import.what {
                        if what.eq(name) {
                            prefixed.push_str(&Self::model_to_package(&import.from));
                            prefixed.push('.');
                            break 'outer;
                        }
                    }
                }
                prefixed.push_str(&name);
                prefixed
            }
            ProtobufType::Repeated(inner) => {
                format!("repeated {}", Self::role_to_full_type(inner, model))
            }
            r => r.to_string(),
        }
    }

    pub fn variant_name(name: &str) -> String {
        name.replace("-", "_").to_uppercase().to_string()
    }

    pub fn field_name(name: &str) -> String {
        name.replace("-", "_")
    }

    pub fn model_file_name(model: &str) -> String {
        let mut name = Self::model_name(model, '_');
        name.push_str(".proto");
        name
    }
    pub fn model_name(model: &str, separator: char) -> String {
        let mut out = String::new();
        let mut prev_lowered = false;
        let mut chars = model.chars().peekable();
        while let Some(c) = chars.next() {
            let mut lowered = false;
            if c.is_uppercase() {
                if !out.is_empty() {
                    if !prev_lowered {
                        out.push(separator);
                    } else if let Some(next) = chars.peek() {
                        if next.is_lowercase() {
                            out.push(separator);
                        }
                    }
                }
                lowered = true;
                out.push_str(&c.to_lowercase().to_string());
            } else if c == '-' {
                out.push(separator);
            } else {
                out.push(c);
            }
            prev_lowered = lowered;
        }
        out
    }

    pub fn model_to_package(model: &str) -> String {
        Self::model_name(&model.replace("_", "."), '.')
    }
}
