use std::fmt::Error as FmtError;
use std::fmt::Write;

use model::Definition;
use model::Model;
use model::Role;

use gen::Generator;

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
    models: Vec<Model>,
}

impl Generator for ProtobufDefGenerator {
    type Error = Error;

    fn add_model(&mut self, model: Model) {
        self.models.push(model);
    }

    fn models(&self) -> &[Model] {
        &self.models[..]
    }

    fn models_mut(&mut self) -> &mut [Model] {
        &mut self.models[..]
    }

    fn to_string(&self) -> Result<Vec<(String, String)>, <Self as Generator>::Error> {
        let mut files = Vec::new();
        for model in self.models.iter() {
            files.push(Self::generate_file(model)?);
        }
        Ok(files)
    }
}

impl ProtobufDefGenerator {
    pub fn generate_file(model: &Model) -> Result<(String, String), Error> {
        let file_name = Self::model_file_name(&model.name);
        let mut content = String::new();
        Self::append_header(&mut content, model)?;
        Self::append_imports(&mut content, model)?;
        for definition in model.definitions.iter() {
            Self::append_definition(&mut content, model, definition)?;
        }
        Ok((file_name, content))
    }

    pub fn append_header(target: &mut Write, model: &Model) -> Result<(), Error> {
        writeln!(target, "syntax = 'proto3';")?;
        writeln!(target, "package {};", Self::model_to_package(&model.name))?;
        writeln!(target)?;
        Ok(())
    }

    pub fn append_imports(target: &mut Write, model: &Model) -> Result<(), Error> {
        for import in model.imports.iter() {
            writeln!(target, "import '{}';", Self::model_file_name(&import.from))?;
        }
        writeln!(target)?;
        Ok(())
    }

    pub fn append_definition(
        target: &mut Write,
        model: &Model,
        definition: &Definition,
    ) -> Result<(), Error> {
        match definition {
            Definition::Enumerated(name, variants) => {
                writeln!(target, "enum {} {{", name)?;
                for (tag, variant) in variants.iter().enumerate() {
                    Self::append_variant(target, &variant, tag)?;
                }
                writeln!(target, "}}")?;
            }
            Definition::Sequence(name, fields) => {
                writeln!(target, "message {} {{", name)?;
                for (prev_tag, field) in fields.iter().enumerate() {
                    Self::append_field(target, model, &field.name, &field.role, prev_tag + 1)?;
                }
                writeln!(target, "}}")?;
            }
            Definition::SequenceOf(name, aliased) => {
                writeln!(target, "message {} {{", name)?;
                writeln!(
                    target,
                    "    repeated {} values = 1;",
                    Self::role_to_full_type(&aliased, model)
                )?;
                writeln!(target, "}}")?;
            }
            Definition::Choice(name, variants) => {
                writeln!(target, "message {} {{", name)?;
                writeln!(target, "    oneof value {{")?;
                for (prev_tag, (name, role)) in variants.iter().enumerate() {
                    write!(target, "    ")?;
                    Self::append_field(target, model, &name, role, prev_tag + 1)?;
                }
                writeln!(target, "    }}")?;
                writeln!(target, "}}")?;
            }
        }
        Ok(())
    }

    pub fn append_field(
        target: &mut Write,
        model: &Model,
        name: &str,
        role: &Role,
        tag: usize,
    ) -> Result<(), Error> {
        writeln!(
            target,
            "    {} {} = {};",
            Self::role_to_full_type(role, model),
            Self::field_name(name),
            tag
        )?;
        Ok(())
    }

    pub fn append_variant(target: &mut Write, variant: &str, tag: usize) -> Result<(), Error> {
        writeln!(target, "    {} = {};", Self::variant_name(&variant), tag)?;
        Ok(())
    }

    pub fn role_to_full_type(role: &Role, model: &Model) -> String {
        let type_name = match role {
            Role::Custom(name) => {
                let mut prefixed = String::new();
                'outer: for import in model.imports.iter() {
                    for what in import.what.iter() {
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
            r => r.clone().into_protobuf().to_string(),
        };
        type_name
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
        let mut chars = model.clone().chars().peekable();
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
        Self::model_name(model, '.')
    }
}
