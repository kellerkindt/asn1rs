use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;
use gen::rust::GeneratorSupplement;
use gen::rust::RustCodeGenerator;
use model::sql::ToSql;
use model::Definition;
use model::Model;
use model::Rust;
use model::RustType;

const ERROR_TYPE: &str = "PsqlError";
const ROW_TYPE: &str = "PsqlRow";
const TRAIT_PSQL_REPRESENTABLE: &str = "PsqlRepresentable";
const TRAIT_PSQL_INSERTABLE: &str = "PsqlInsertable";
const TRAIT_PSQL_QUERYABLE: &str = "PsqlQueryable";

pub struct PsqlInserter;
impl GeneratorSupplement<Rust> for PsqlInserter {
    fn add_imports(&self, scope: &mut Scope) {
        scope.import("asn1c::io::psql", &format!("Error as {}", ERROR_TYPE));
        scope.import("asn1c::io::psql", &format!("Row as {}", ROW_TYPE));
        scope.import(
            "asn1c::io::psql",
            &format!("Representable as {}", TRAIT_PSQL_REPRESENTABLE),
        );
        scope.import(
            "asn1c::io::psql",
            &format!("Insertable as {}", TRAIT_PSQL_INSERTABLE),
        );
        scope.import(
            "asn1c::io::psql",
            &format!("Queryable as {}", TRAIT_PSQL_QUERYABLE),
        );
        scope.import("asn1c::io::psql", "Transaction");
    }

    fn impl_supplement(&self, scope: &mut Scope, definition: &Definition<Rust>) {
        Self::impl_representable(scope, definition);
        Self::impl_insertable(scope, definition);
        Self::impl_queryable(scope, definition);
    }
}

impl PsqlInserter {
    fn new_representable_impl<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Impl {
        scope.new_impl(name).impl_trait(TRAIT_PSQL_REPRESENTABLE)
    }

    fn new_insertable_impl<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Impl {
        scope.new_impl(name).impl_trait(TRAIT_PSQL_INSERTABLE)
    }

    fn new_queryable_impl<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Impl {
        scope.new_impl(name).impl_trait(TRAIT_PSQL_QUERYABLE)
    }

    fn impl_representable(scope: &mut Scope, Definition(name, _rust): &Definition<Rust>) {
        let implementation = Self::new_representable_impl(scope, &name);
        Self::impl_table_name(Self::new_table_name_fn(implementation), name);
    }

    fn impl_insertable(scope: &mut Scope, Definition(name, rust): &Definition<Rust>) {
        let implementation = Self::new_insertable_impl(scope, &name);
        match rust {
            Rust::Struct(fields) => {
                Self::impl_struct_insert_statement(
                    Self::new_insert_statement_fn(implementation),
                    name,
                    &fields[..],
                );
                Self::impl_struct_insert_fn(
                    Self::new_insert_fn(implementation, true),
                    name,
                    &fields[..],
                );
            }
            Rust::DataEnum(fields) => {
                Self::impl_data_enum_insert_statement(
                    Self::new_insert_statement_fn(implementation),
                    name,
                    &fields[..],
                );
                Self::impl_data_enum_insert_fn(
                    Self::new_insert_fn(implementation, true),
                    name,
                    &fields[..],
                );
            }
            Rust::Enum(_) => {
                Self::impl_enum_insert_statement(Self::new_insert_statement_fn(implementation));
                Self::impl_enum_insert_fn(Self::new_insert_fn(implementation, false));
            }
            Rust::TupleStruct(rust) => {
                Self::impl_tuple_insert_statement(
                    Self::new_insert_statement_fn(implementation),
                    name,
                );
                Self::impl_tuple_insert_fn(Self::new_insert_fn(implementation, true), name, rust);
            }
        }
    }

    fn new_table_name_fn(implementation: &mut Impl) -> &mut Function {
        implementation
            .new_fn("table_name")
            .arg_ref_self()
            .ret("&'static str")
    }

    fn new_insert_statement_fn(implementation: &mut Impl) -> &mut Function {
        implementation
            .new_fn("insert_statement")
            .arg_ref_self()
            .ret("&'static str")
    }

    fn new_insert_fn(implementation: &mut Impl, using_transaction: bool) -> &mut Function {
        implementation
            .new_fn("insert_with")
            .arg_ref_self()
            .arg(
                if using_transaction {
                    "transaction"
                } else {
                    "_"
                },
                "&Transaction",
            ).ret(&format!("Result<i32, {}>", ERROR_TYPE))
    }

    fn impl_table_name(function: &mut Function, name: &str) {
        function.line(&format!("\"{}\"", name));
    }

    fn impl_struct_insert_statement(
        function: &mut Function,
        name: &str,
        fields: &[(String, RustType)],
    ) {
        if fields.is_empty() {
            Self::impl_tuple_insert_statement(function, name);
        } else {
            function.line(&format!(
                "\"INSERT INTO {}({}) VALUES({}) RETURNING id\"",
                name,
                fields
                    .iter()
                    .map(|(name, _)| Model::sql_column_name(name))
                    .collect::<Vec<String>>()
                    .join(", "),
                fields
                    .iter()
                    .enumerate()
                    .map(|(num, _)| format!("${}", num + 1))
                    .collect::<Vec<String>>()
                    .join(", "),
            ));
        }
    }

    fn impl_data_enum_insert_statement(
        function: &mut Function,
        name: &str,
        fields: &[(String, RustType)],
    ) {
        if fields.is_empty() {
            Self::impl_tuple_insert_statement(function, name);
        } else {
            function.line(&format!(
                "\"INSERT INTO {}({}) VALUES({}) RETURNING id\"",
                name,
                fields
                    .iter()
                    .map(|(name, _)| RustCodeGenerator::rust_module_name(name))
                    .collect::<Vec<String>>()
                    .join(", "),
                fields
                    .iter()
                    .enumerate()
                    .map(|(num, _)| format!("${}", num + 1))
                    .collect::<Vec<String>>()
                    .join(", "),
            ));
        }
    }

    fn impl_enum_insert_statement(function: &mut Function) {
        function.line("\"\"");
    }

    fn impl_tuple_insert_statement(function: &mut Function, name: &str) {
        function.line(&format!(
            "\"INSERT INTO {} DEFAULT VALUES RETURNING id\"",
            name
        ));
    }

    fn impl_struct_insert_fn(function: &mut Function, _name: &str, fields: &[(String, RustType)]) {
        let mut variables = Vec::with_capacity(fields.len());
        for (name, rust) in fields {
            let name = RustCodeGenerator::rust_field_name(name, true);
            let sql_primitive = Self::is_sql_primitive(rust);
            variables.push(format!("&{}", name));
            if sql_primitive {
                function.line(&format!(
                    "let {} = {}self.{};",
                    name,
                    if !rust.is_primitive() { "&" } else { "" },
                    name,
                ));
                if let Some(wrap) = Self::wrap_for_insert_in_as_or_from_if_required(&name, rust) {
                    function.line(format!("let {} = {};", name, wrap,));
                }
            } else {
                function.line(&format!(
                    "let {} = {}self.{}{}.insert_with(transaction)?{};",
                    name,
                    if let RustType::Option(_) = rust {
                        "if let Some(ref value) = "
                    } else {
                        ""
                    },
                    name,
                    if let RustType::Option(_) = rust {
                        " { Some(value"
                    } else {
                        ""
                    },
                    if let RustType::Option(_) = rust {
                        ") } else { None }"
                    } else {
                        ""
                    }
                ));
            };
        }
        function.line("let statement = transaction.prepare_cached(self.insert_statement())?;");
        function.line(format!(
            "let result = statement.query(&[{}])?;",
            variables.join(", ")
        ));
        function.line(&format!("{}::expect_returned_index(&result)", ERROR_TYPE));
    }

    fn wrap_for_insert_in_as_or_from_if_required(name: &str, rust: &RustType) -> Option<String> {
        let inner_sql = rust.clone().into_inner_type().to_sql();
        let inner_rust = rust.clone().into_inner_type();
        if !inner_sql.to_rust().into_inner_type().similar(&inner_rust) {
            Some({
                let rust_from_sql = inner_sql.to_rust().into_inner_type();
                let as_target = rust_from_sql.to_string();
                let use_from_instead_of_as =
                    rust_from_sql.is_primitive() && rust_from_sql > inner_rust;
                if let RustType::Option(_) = rust {
                    if use_from_instead_of_as {
                        format!("{}.map({}::from)", name, as_target)
                    } else {
                        format!("{}.map(|v| v as {})", name, as_target)
                    }
                } else if use_from_instead_of_as {
                    format!("{}::from({})", as_target, name)
                } else {
                    format!("{} as {}", name, as_target)
                }
            })
        } else {
            None
        }
    }

    fn wrap_for_query_in_as_or_from_if_required(name: &str, rust: &RustType) -> Option<String> {
        let inner_sql = rust.clone().into_inner_type().to_sql();
        let inner_rust = rust.clone().into_inner_type();
        if !inner_sql.to_rust().into_inner_type().similar(&inner_rust) {
            Some({
                let as_target = inner_rust.to_string();
                if let RustType::Option(_) = rust {
                    format!("{}.map(|v| v as {})", name, as_target)
                } else {
                    format!("{} as {}", name, as_target)
                }
            })
        } else {
            None
        }
    }

    fn impl_data_enum_insert_fn(
        function: &mut Function,
        name: &str,
        fields: &[(String, RustType)],
    ) {
        let mut variables = Vec::with_capacity(fields.len());
        for (variant, rust) in fields {
            let variable = RustCodeGenerator::rust_field_name(
                &RustCodeGenerator::rust_module_name(variant),
                true,
            );
            let sql_primitive = Self::is_sql_primitive(rust);
            variables.push(format!("&{}", variable));
            let mut block_if = Block::new(&format!(
                "let {} = if let {}::{}(value) = self",
                variable, name, variant
            ));

            if sql_primitive {
                block_if.line("Some(value)");
            } else {
                block_if.line("Some(value.insert_with(transaction)?)");
            };

            block_if.after(" else { None };");
            function.push_block(block_if);
        }
        function.line("let statement = transaction.prepare_cached(self.insert_statement())?;");
        function.line(format!(
            "let result = statement.query(&[{}])?;",
            variables.join(", ")
        ));
        function.line(&format!("{}::expect_returned_index(&result)", ERROR_TYPE));
    }

    fn impl_enum_insert_fn(function: &mut Function) {
        function.line("Ok(self.value_index() as i32)");
    }

    fn impl_tuple_insert_fn(function: &mut Function, name: &str, rust: &RustType) {
        function.line("let statement = transaction.prepare_cached(self.insert_statement())?;");
        function.line("let result = statement.query(&[])?;");
        function.line(&format!(
            "let list = {}::expect_returned_index(&result)?;",
            ERROR_TYPE
        ));
        function.line(format!(
            "let statement = transaction.prepare_cached(\"{}\")?;",
            Self::list_entry_insert_statement(name)
        ));
        let mut block_for = Block::new("for value in &self.0");
        let sql_primitive = Self::is_sql_primitive(rust);
        if !sql_primitive {
            block_for.line("let value = value.insert_with(transaction)?;");
        } else {
            let inner_sql = rust.clone().into_inner_type().to_sql();
            let inner_rust = rust.clone().into_inner_type();
            if !inner_sql.to_rust().into_inner_type().similar(&inner_rust) {
                let rust_from_sql = inner_sql.to_rust().into_inner_type();
                let as_target = rust_from_sql.to_string();
                let use_from_instead_of_as =
                    rust_from_sql.is_primitive() && rust_from_sql > inner_rust;
                block_for.line(format!(
                    "let value = {};",
                    if let RustType::Option(_) = rust {
                        if use_from_instead_of_as {
                            format!("value.map({}::from)", as_target)
                        } else {
                            format!("value.map(|v| v as {})", as_target)
                        }
                    } else if use_from_instead_of_as {
                        format!("{}::from(*value)", as_target)
                    } else {
                        format!("*value as {}", as_target)
                    },
                ));
            }
        }
        block_for.line("statement.execute(&[&list, &value])?;");
        function.push_block(block_for);
        function.line("Ok(list)");
    }

    fn impl_queryable(scope: &mut Scope, Definition(name, rust): &Definition<Rust>) {
        let implementation = Self::new_queryable_impl(scope, name);
        match rust {
            Rust::Struct(fields) => {
                Self::impl_query_statement(Self::new_query_statement_fn(implementation), name);
                Self::impl_struct_query_fn(Self::new_query_fn(implementation, true), name);
                Self::impl_struct_load_fn(
                    Self::new_load_fn(
                        implementation,
                        fields.iter().any(|(_, rust)| !Self::is_sql_primitive(rust)),
                    ),
                    name,
                    &fields[..],
                );
            }
            Rust::DataEnum(fields) => {
                Self::impl_query_statement(Self::new_query_statement_fn(implementation), name);
                Self::impl_data_enum_query_fn(Self::new_query_fn(implementation, true), name);
                Self::impl_data_enum_load_fn(
                    Self::new_load_fn(implementation, true),
                    name,
                    &fields[..],
                );
            }
            Rust::Enum(variants) => {
                Self::impl_empty_query_statement(Self::new_query_statement_fn(implementation));
                Self::impl_enum_query_fn(
                    Self::new_query_fn(implementation, false),
                    name,
                    &variants[..],
                );
                Self::impl_enum_load_fn(Self::new_load_fn(implementation, true), name);
            }
            Rust::TupleStruct(rust) => {
                Self::impl_tupl_query_statement(
                    Self::new_query_statement_fn(implementation),
                    name,
                    &rust.clone().into_inner_type().to_string(),
                );
                Self::impl_tupl_struct_query_fn(
                    Self::new_query_fn(implementation, true),
                    name,
                    rust,
                );
                Self::impl_tupl_struct_load_fn(Self::new_load_fn(implementation, true), name);
            }
        }
    }

    fn impl_query_statement(func: &mut Function, name: &str) {
        func.line(&format!("\"SELECT * FROM {} WHERE id = $1\"", name));
    }

    fn impl_empty_query_statement(func: &mut Function) {
        func.line("\"\"");
    }

    fn impl_tupl_query_statement(func: &mut Function, name: &str, inner: &str) {
        func.line(&format!(
            "\"{}\"",
            Self::list_entry_query_statement(name, inner)
        ));
    }

    fn impl_struct_query_fn(func: &mut Function, name: &str) {
        func.line("let statement = transaction.prepare_cached(Self::query_statement())?;");
        func.line("let rows = statement.query(&[&id])?;");
        func.line(&format!(
            "Ok({}::load_from(transaction, &rows.iter().next().ok_or({}::NoResult)?)?)",
            name, ERROR_TYPE,
        ));
    }
    fn impl_struct_load_fn(func: &mut Function, name: &str, variants: &[(String, RustType)]) {
        let mut block = Block::new(&format!("Ok({}", name));

        for (index, (name, rust)) in variants.iter().enumerate() {
            let inner = rust.clone().into_inner_type();
            if Self::is_sql_primitive(&inner) {
                let load = format!(
                    "row.get_opt::<usize, {}>({}).ok_or({}::NoResult)??",
                    rust.to_sql().to_rust().to_string(),
                    index,
                    ERROR_TYPE,
                );
                block.line(&format!(
                    "{}: {},",
                    RustCodeGenerator::rust_field_name(name, true),
                    Self::wrap_for_query_in_as_or_from_if_required(&load, rust,).unwrap_or(load)
                ));
            } else {
                let load = if let RustType::Option(_) = rust {
                    format!(
                        "if let Some(id) = row.get_opt::<usize, Option<i32>>({}).ok_or({}::NoResult)?? {{\
                         Some({}::query_with(transaction, id)?)\
                         }} else {{\
                         None\
                         }}",
                        index,
                        ERROR_TYPE,
                        inner.to_string(),
                    )
                } else {
                    format!(
                        "{}::query_with(transaction, row.get_opt({}).ok_or({}::NoResult)??)?",
                        inner.to_string(),
                        index,
                        ERROR_TYPE,
                    )
                };
                block.line(&format!(
                    "{}: {},",
                    RustCodeGenerator::rust_field_name(name, true),
                    load
                ));
            }
        }

        block.after(")");
        func.push_block(block);
    }

    fn impl_data_enum_query_fn(func: &mut Function, name: &str) {
        func.line("let statement = transaction.prepare_cached(Self::query_statement())?;");
        func.line("let rows = statement.query(&[&id])?;");
        func.line(&format!(
            "Ok({}::load_from(transaction, &rows.iter().next().ok_or({}::NoResult)?)?)",
            name, ERROR_TYPE
        ));
    }
    fn impl_data_enum_load_fn(func: &mut Function, name: &str, variants: &[(String, RustType)]) {
        func.line(&format!(
            "let (index, id) = {}::first_not_null(row, &[{}])?;",
            ERROR_TYPE,
            variants
                .iter()
                .enumerate()
                .map(|e| format!("{}", e.0 + 1))
                .collect::<Vec<String>>()
                .join(", ")
        ));
        let mut block = Block::new("match index");
        for (index, (variant, rust)) in variants.iter().enumerate() {
            block.line(&format!(
                "{} => Ok({}::{}({}::query_with(transaction, id)?)),",
                index,
                name,
                variant,
                rust.clone().into_inner_type().to_string(),
            ));
        }
        block.line(&format!("_ => Err({}::NoResult),", ERROR_TYPE));
        func.push_block(block);
    }

    fn impl_enum_query_fn(func: &mut Function, name: &str, variants: &[String]) {
        let mut block = Block::new("match id");
        for (index, variant) in variants.iter().enumerate() {
            block.line(&format!("{} => Ok({}::{}),", index, name, variant));
        }
        block.line(&format!("_ => Err({}::NoResult),", ERROR_TYPE));
        func.push_block(block);
    }

    fn impl_enum_load_fn(func: &mut Function, name: &str) {
        func.line(&format!(
            "Ok({}::query_with(transaction, row.get_opt::<usize, i32>(0).ok_or({}::NoResult)??)?)",
            name, ERROR_TYPE,
        ));
    }

    fn impl_tupl_struct_query_fn(func: &mut Function, name: &str, rust: &RustType) {
        func.line("let statement = transaction.prepare_cached(Self::query_statement())?;");
        func.line("let rows = statement.query(&[&id])?;");
        func.line("let mut values = Vec::with_capacity(rows.len());");
        let inner = rust.clone().into_inner_type();
        if Self::is_sql_primitive(&inner) {
            let mut block = Block::new("for (number, row) in rows.iter().enumerate()");
            let from_sql = inner.to_sql().to_rust();
            let load = format!(
                "row.get_opt::<usize, {}>(number).ok_or({}::NoResult)??",
                from_sql.to_string(),
                ERROR_TYPE,
            );
            block.line(&format!(
                "values.push({});",
                Self::wrap_for_query_in_as_or_from_if_required(&load, rust).unwrap_or(load)
            ));
            func.push_block(block);
        } else {
            let mut block = Block::new("for row in rows.iter()");
            block.line(&format!(
                "values.push({}::load_from(transaction, &row)?);",
                inner.to_string()
            ));
            func.push_block(block);
        }
        func.line(&format!("Ok({}(values))", name));
    }

    fn impl_tupl_struct_load_fn(func: &mut Function, name: &str) {
        func.line(&format!(
            "Ok({}::query_with(transaction, row.get_opt(0).ok_or({}::NoResult)??)?)",
            name, ERROR_TYPE,
        ));
    }

    fn new_query_statement_fn(implementation: &mut Impl) -> &mut Function {
        implementation.new_fn("query_statement").ret("&'static str")
    }

    fn new_query_fn(implementation: &mut Impl, using_transaction: bool) -> &mut Function {
        implementation
            .new_fn("query_with")
            .arg(
                if using_transaction {
                    "transaction"
                } else {
                    "_"
                },
                "&Transaction",
            ).arg("id", "i32")
            .ret(&format!("Result<Self, {}>", ERROR_TYPE))
    }

    fn new_load_fn(implementation: &mut Impl, using_transaction: bool) -> &mut Function {
        implementation
            .new_fn("load_from")
            .arg(
                if using_transaction {
                    "transaction"
                } else {
                    "_"
                },
                "&Transaction",
            ).arg("row", &format!("&{}", ROW_TYPE))
            .ret(&format!("Result<Self, {}>", ERROR_TYPE))
    }

    fn is_sql_primitive(rust: &RustType) -> bool {
        match rust.clone().into_inner_type() {
            RustType::String => true,
            RustType::VecU8 => true,
            r => r.is_primitive(),
        }
    }

    fn list_entry_insert_statement(name: &str) -> String {
        format!("INSERT INTO {}ListEntry(list, value) VALUES ($1, $2)", name)
    }

    fn list_entry_query_statement(name: &str, inner: &str) -> String {
        format!(
            "SELECT * FROM {} WHERE {}.id = {}ListEntry.value AND {}ListEntry.list = $1",
            inner, inner, name, name
        )
    }
}
