use crate::gen::Generator;
use crate::model::sql::Column;
use crate::model::sql::Constraint;
use crate::model::sql::Sql;
use crate::model::Definition;
use crate::model::Model;
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
            let mut drop = String::new();
            let mut create = String::new();
            for Definition(name, sql) in &model.definitions {
                writeln!(create)?;
                match sql {
                    Sql::Table(columns, constraints) => {
                        // TODO
                        writeln!(drop, "DROP TABLE IF EXISTS {} CASCADE;", name)?;
                        Self::append_create_table(&mut create, name, columns, constraints)?;
                    }
                    Sql::Enum(variants) => {
                        // TODO
                        writeln!(drop, "DROP TABLE IF EXISTS {} CASCADE;", name)?;
                        Self::append_create_enum(&mut create, name, variants)?
                    }
                    Sql::Index(table, columns) => {
                        Self::append_index(&mut create, name, table, &columns[..])?;
                    }
                    Sql::AbandonChildrenFunction(table, children) => {
                        Self::append_abandon_children(&mut create, table, name, &children[..])?;
                    }
                    Sql::SilentlyPreventAnyDelete(table) => {
                        Self::append_silently_prevent_any_delete(&mut create, name, table)?;
                    }
                }
            }
            drop.push_str(&create);
            files.push((format!("{}.sql", model.name), drop));
        }
        Ok(files)
    }
}

impl SqlDefGenerator {
    pub fn append_create_table(
        target: &mut Write,
        name: &str,
        columns: &[Column],
        constraints: &[Constraint],
    ) -> Result<(), Error> {
        writeln!(target, "CREATE TABLE {} (", name)?;
        for (index, column) in columns.iter().enumerate() {
            Self::append_column_statement(target, column)?;
            if index + 1 < columns.len() || !constraints.is_empty() {
                write!(target, ",")?;
            }
            writeln!(target)?;
        }
        for (index, constraint) in constraints.iter().enumerate() {
            Self::append_constraint(target, constraint)?;
            if index + 1 < constraints.len() {
                write!(target, ",")?;
            }
            writeln!(target)?;
        }
        writeln!(target, ");")?;
        Ok(())
    }

    pub fn append_column_statement(target: &mut Write, column: &Column) -> Result<(), Error> {
        write!(target, "    {} {}", column.name, column.sql.to_string())?;
        if column.primary_key {
            write!(target, " PRIMARY KEY")?;
        }
        Ok(())
    }

    pub fn append_create_enum(
        target: &mut Write,
        name: &str,
        variants: &[String],
    ) -> Result<(), Error> {
        writeln!(target, "CREATE TABLE {} (", name)?;
        writeln!(target, "    id SERIAL PRIMARY KEY,")?;
        writeln!(target, "    name TEXT NOT NULL")?;
        writeln!(target, ");")?;

        writeln!(target, "INSERT INTO {} (id, name) VALUES", name)?;
        for (index, variant) in variants.iter().enumerate() {
            write!(target, "    ({}, '{}')", index, variant)?;
            if index + 1 < variants.len() {
                write!(target, ", ")?;
            } else {
                write!(target, ";")?;
            }
            writeln!(target)?;
        }
        Ok(())
    }

    fn append_constraint(target: &mut Write, constraint: &Constraint) -> Result<(), Error> {
        match constraint {
            Constraint::CombinedPrimaryKey(columns) => {
                write!(target, "    PRIMARY KEY({})", columns.join(", "))?;
            }
            Constraint::OneNotNull(columns) => {
                write!(
                    target,
                    "    CHECK (num_nonnulls({}) = 1)",
                    columns.join(", ")
                )?;
            }
        }
        Ok(())
    }

    fn append_index(
        target: &mut Write,
        name: &str,
        table: &str,
        columns: &[String],
    ) -> Result<(), Error> {
        writeln!(
            target,
            "CREATE INDEX {} ON {}({});",
            name,
            table,
            columns.join(", ")
        )?;
        Ok(())
    }

    fn append_abandon_children(
        target: &mut Write,
        table: &str,
        name: &str,
        children: &[(String, String, String)],
    ) -> Result<(), Error> {
        writeln!(
            target,
            "CREATE OR REPLACE FUNCTION {}() RETURNS TRIGGER AS",
            name
        )?;
        writeln!(target, "$$ BEGIN")?;
        for (column, other_table, other_column) in children {
            writeln!(
                target,
                "    DELETE FROM {} WHERE {} = OLD.{};",
                other_table, other_column, column
            )?;
        }
        writeln!(target, "    RETURN NULL;")?;
        writeln!(target, "END; $$ LANGUAGE plpgsql;")?;
        writeln!(
            target,
            "CREATE TRIGGER OnDelete{} AFTER DELETE ON {}",
            name, table
        )?;
        writeln!(target, "    FOR EACH ROW")?;
        writeln!(target, "    EXECUTE PROCEDURE {}();", name)?;
        Ok(())
    }

    fn append_silently_prevent_any_delete(
        target: &mut Write,
        name: &str,
        table: &str,
    ) -> Result<(), Error> {
        writeln!(target, "CREATE RULE {} AS ON DELETE TO {}", name, table)?;
        writeln!(target, "    DO INSTEAD NOTHING;")?;
        Ok(())
    }
}
