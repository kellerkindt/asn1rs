use gen::Generator;
use model::sql::Column;
use model::sql::Constraint;
use model::sql::Sql;
use model::Definition;
use model::Model;
use std::fmt::Write;

#[derive(Debug)]
pub enum Error {
    Fmt(::std::fmt::Error),
}

impl From<::std::fmt::Error> for Error {
    fn from(e: ::std::fmt::Error) -> Self {
        Error::Fmt(e)
    }
}

#[derive(Debug, Default)]
pub struct SqlDefGenerator {
    models: Vec<Model<Sql>>,
}

impl Generator<Sql> for SqlDefGenerator {
    type Error = Error;

    fn add_model(&mut self, model: Model<Sql>) {
        self.models.push(model);
    }

    fn models(&self) -> &[Model<Sql>] {
        &self.models
    }

    fn models_mut(&mut self) -> &mut [Model<Sql>] {
        &mut self.models
    }

    fn to_string(&self) -> Result<Vec<(String, String)>, <Self as Generator<Sql>>::Error> {
        let mut files = Vec::with_capacity(self.models.len());
        for model in &self.models {
            let mut string = String::new();
            for Definition(name, sql) in &model.definitions {
                match sql {
                    Sql::Table((columns, constraints)) => {
                        Self::append_create_table(&mut string, name, columns, constraints)?;
                    }
                    Sql::Enum(variants) => Self::append_create_enum(&mut string, name, variants)?,
                }
            }
            files.push((format!("{}.sql", model.name), string));
        }
        Ok(files)
    }
}

impl SqlDefGenerator {
    pub fn append_create_table(
        target: &mut Write,
        name: &String,
        columns: &[Column],
        constraints: &[Constraint],
    ) -> Result<(), Error> {
        writeln!(target, "CREATE TABLE {} (", name)?;
        for (index, column) in columns.iter().enumerate() {
            Self::append_column_statement(target, column)?;
            if index + 1 < columns.len() {
                write!(target, ",");
            }
            writeln!(target);
        }
        writeln!(target, ");")?;
        Ok(())
    }

    pub fn append_column_statement(target: &mut Write, column: &Column) -> Result<(), Error> {
        write!(target, "    {} {}", column.name, column.sql.to_string())?;
        if column.primary_key {
            write!(target, " PRIMARY KEY");
        }
        Ok(())
    }

    pub fn append_create_enum(
        target: &mut Write,
        name: &String,
        variants: &[String],
    ) -> Result<(), Error> {
        write!(target, "CREATE TYPE {} AS ENUM (", name)?;
        for (index, variant) in variants.iter().enumerate() {
            write!(target, "'{}'", variant)?;
            if index + 1 < variants.len() {
                write!(target, ", ")?;
            }
        }
        writeln!(target, ");");
        Ok(())
    }
}
