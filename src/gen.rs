use codegen::Scope;
use codegen::Type;

use model::Definition;
use model::Field;
use model::Model;
use model::Role;

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
            let mut string = model.name.to_lowercase();
            string.push_str(".rs");
            string
        };

        let mut scope = Scope::new();
        scope.import("buffer", "BitBuffer");

        for import in model.imports.iter() {
            for what in import.what.iter() {
                scope.import(&import.from, &what);
            }
        }

        for definition in model.definitions.iter() {
            match definition {
                Definition::SequenceOf(name, role) => {
                    Self::new_struct(&mut scope, name).field("values", Self::role_to_type(role));
                }
                Definition::Sequence(name, fields) => {
                    let mut new_struct = Self::new_struct(&mut scope, name);
                    for field in fields.iter() {
                        new_struct.field(
                            &Self::rust_field_name(&field.name),
                            Self::role_to_type(&field.role),
                        );
                    }
                }
            }
        }

        Ok((file, scope.to_string()))
    }

    fn role_to_type(role: &Role) -> Type {
        let type_name = match role {
            Role::Boolean => "bool".into(),
            Role::Integer(range) => {
                if let Some((start, end)) = range {
                    match (end - start) {
                        0x00_00_00_00__00_00_00_00...0x00_00_00_00__00_00_00_FF => "i8".into(),
                        0x00_00_00_00__00_00_00_00...0x00_00_00_00__00_00_FF_FF => "i16".into(),
                        0x00_00_00_00__00_00_00_00...0x00_00_00_00__FF_FF_FF_FF => "i32".into(),
                        _ => "i64".into(),
                    }
                } else {
                    "i32".into()
                }
            }
            Role::Custom(name) => name.clone(),
        };
        Type::new(&type_name)
    }

    fn rust_field_name(name: &str) -> String {
        name.replace("-", "_")
    }

    fn new_struct<'a>(scope: &'a mut Scope, name: &str) -> &'a mut ::codegen::Struct {
        scope
            .new_struct(name)
            .vis("pub")
            .derive("Default")
            .derive("Debug")
    }
}
