use crate::gen::rust::GeneratorSupplement;
use crate::gen::rust::RustCodeGenerator;
use crate::model::sql::Sql;
use crate::model::sql::ToSql;
use crate::model::Definition;
use crate::model::Model;
use crate::model::Rust;
use crate::model::RustType;
use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;

const ERROR_TYPE: &str = "PsqlError";
const ROW_TYPE: &str = "PsqlRow";
const TRAIT_PSQL_REPRESENTABLE: &str = "PsqlRepresentable";
const TRAIT_PSQL_INSERTABLE: &str = "PsqlInsertable";
const TRAIT_PSQL_QUERYABLE: &str = "PsqlQueryable";

pub struct PsqlInserter;
impl GeneratorSupplement<Rust> for PsqlInserter {
    fn add_imports(&self, scope: &mut Scope) {
        scope.import("asn1rs::io::psql", &format!("Error as {}", ERROR_TYPE));
        scope.import("asn1rs::io::psql", &format!("Row as {}", ROW_TYPE));
        scope.import(
            "asn1rs::io::psql",
            &format!("Representable as {}", TRAIT_PSQL_REPRESENTABLE),
        );
        scope.import(
            "asn1rs::io::psql",
            &format!("Insertable as {}", TRAIT_PSQL_INSERTABLE),
        );
        scope.import(
            "asn1rs::io::psql",
            &format!("Queryable as {}", TRAIT_PSQL_QUERYABLE),
        );
        scope.import("asn1rs::io::psql", "Transaction");
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
            )
            .ret(&format!("Result<i32, {}>", ERROR_TYPE))
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
                    .filter_map(|(name, field)| if Model::<Sql>::is_vec(field) {
                        None
                    } else {
                        Some(name)
                    })
                    .map(|name| Model::sql_column_name(name))
                    .collect::<Vec<String>>()
                    .join(", "),
                fields
                    .iter()
                    .filter_map(|(name, field)| if Model::<Sql>::is_vec(field) {
                        None
                    } else {
                        Some(name)
                    })
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

    fn impl_struct_insert_fn(
        function: &mut Function,
        struct_name: &str,
        fields: &[(String, RustType)],
    ) {
        let mut variables = Vec::with_capacity(fields.len());
        let mut vecs = Vec::new();
        for (name, rust) in fields {
            let name = RustCodeGenerator::rust_field_name(name, true);
            let sql_primitive = Self::is_sql_primitive(rust);
            let is_vec = Model::<Sql>::is_vec(rust);

            if !is_vec {
                variables.push(format!("&{}", name));
            } else {
                vecs.push((name.clone(), rust.clone()));
                continue;
            }
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
            } else if !is_vec {
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
        if vecs.is_empty() {
            function.line(&format!("{}::expect_returned_index(&result)", ERROR_TYPE));
        } else {
            function.line(&format!(
                "let index = {}::expect_returned_index(&result)?;",
                ERROR_TYPE
            ));
            for (name, field) in vecs {
                let mut block = Block::new("");
                block.line(&format!(
                    "let statement = transaction.prepare_cached(\"{}\")?;",
                    &Self::struct_list_entry_insert_statement(&struct_name, &name),
                ));
                block.push_block(Self::list_insert_for_each(&name, &field, "index"));
                function.push_block(block);
            }
            function.line("Ok(index)");
        }
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
        function.push_block(Self::list_insert_for_each("0", rust, "list"));
        function.line("Ok(list)");
    }

    /// Expects a variable called `statement` to be reachable and usable
    fn list_insert_for_each(name: &str, rust: &RustType, list: &str) -> Block {
        let mut block_for = Block::new(&format!(
            "for value in self.{}.iter(){}",
            name,
            if let RustType::Option(_) = rust {
                ".flatten()"
            } else {
                ""
            }
        ));
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
                    if use_from_instead_of_as {
                        format!("{}::from(*value)", as_target)
                    } else {
                        format!("*value as {}", as_target)
                    },
                ));
            }
        }
        block_for.line(format!("statement.execute(&[&{}, &value])?;", list));
        block_for
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
                        fields.iter().any(|(_, rust)| {
                            !Self::is_sql_primitive(rust) || Model::<Sql>::is_vec(rust)
                        }),
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
                    rust,
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

    fn impl_tupl_query_statement(func: &mut Function, name: &str, inner: &RustType) {
        func.line(&format!(
            "\"{}\"",
            Self::list_entry_query_statement(name, inner)
        ));
    }

    fn impl_struct_query_fn(func: &mut Function, name: &str) {
        func.line("let statement = transaction.prepare_cached(Self::query_statement())?;");
        func.line("let rows = statement.query(&[&id])?;");
        func.line(&format!(
            "Ok({}::load_from(transaction, &rows.iter().next().ok_or_else({}::no_result)?)?)",
            name, ERROR_TYPE,
        ));
    }
    fn impl_struct_load_fn(
        func: &mut Function,
        struct_name: &str,
        variants: &[(String, RustType)],
    ) {
        let mut block = Block::new(&format!("Ok({}", struct_name));

        for (index, (name, rust)) in variants.iter().enumerate() {
            if Model::<Sql>::is_vec(rust) {
                let mut load_block = Block::new(&format!(
                    "{}:",
                    RustCodeGenerator::rust_field_name(name, true)
                ));

                load_block.line("let mut vec = Vec::default();");

                load_block.line(&format!(
                        "let rows = transaction.prepare_cached(\"{}\")?.query(&[&row.get_opt::<usize, i32>(0).ok_or_else(PsqlError::no_result)??])?;",

                        if let RustType::Complex(complex) = rust.clone().into_inner_type() {
                            Self::struct_list_entry_select_referenced_value_statement(
                                struct_name,
                                &name,
                                &complex
                            )
                        } else {
                            Self::struct_list_entry_select_value_statement(
                                struct_name, &name
                            )
                        }
                    ));
                let mut rows_foreach = Block::new("for row in rows.iter()");
                if let RustType::Complex(complex) = rust.clone().into_inner_type() {
                    rows_foreach.line(&format!(
                        "vec.push({}::load_from(transaction, &row)?);",
                        complex
                    ));
                } else {
                    let rust = rust.clone().into_inner_type();
                    let sql = rust.to_sql();
                    rows_foreach.line(&format!(
                        "let value = row.get_opt::<usize, {}>({}).ok_or_else({}::no_result)??;",
                        sql.to_rust().to_string(),
                        index + 1,
                        ERROR_TYPE,
                    ));
                    if rust < sql.to_rust() {
                        rows_foreach.line(&format!("let value = value as {};", rust.to_string()));
                    }
                    rows_foreach.line("vec.push(value);");
                }
                load_block.push_block(rows_foreach);

                if let RustType::Option(_) = rust {
                    load_block.line("if vec.is_empty() { None } else { Some(vec) }");
                } else {
                    load_block.line("vec");
                }
                load_block.after(",");
                block.push_block(load_block);
            } else if Self::is_sql_primitive(&rust) {
                let load = format!(
                    "row.get_opt::<usize, {}>({}).ok_or_else({}::no_result)??",
                    rust.to_sql().to_rust().to_string(),
                    index + 1,
                    ERROR_TYPE,
                );
                block.line(&format!(
                    "{}: {},",
                    RustCodeGenerator::rust_field_name(name, true),
                    Self::wrap_for_query_in_as_or_from_if_required(&load, rust,).unwrap_or(load)
                ));
            } else {
                let inner = rust.clone().into_inner_type();
                let load = if let RustType::Option(_) = rust {
                    format!(
                        "if let Some(id) = row.get_opt::<usize, Option<i32>>({}).ok_or_else({}::no_result)?? {{\
                         Some({}::query_with(transaction, id)?)\
                         }} else {{\
                         None\
                         }}",
                        index + 1,
                        ERROR_TYPE,
                        inner.to_string(),
                    )
                } else {
                    format!(
                        "{}::query_with(transaction, row.get_opt({}).ok_or_else({}::no_result)??)?",
                        inner.to_string(),
                        index + 1,
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
            "Ok({}::load_from(transaction, &rows.iter().next().ok_or_else({}::no_result)?)?)",
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
                index + 1,
                name,
                variant,
                rust.clone().into_inner_type().to_string(),
            ));
        }
        block.line(&format!("_ => Err({}::no_result()),", ERROR_TYPE));
        func.push_block(block);
    }

    fn impl_enum_query_fn(func: &mut Function, name: &str, variants: &[String]) {
        let mut block = Block::new("match id");
        for (index, variant) in variants.iter().enumerate() {
            block.line(&format!("{} => Ok({}::{}),", index, name, variant));
        }
        block.line(&format!("_ => Err({}::no_result()),", ERROR_TYPE));
        func.push_block(block);
    }

    fn impl_enum_load_fn(func: &mut Function, name: &str) {
        func.line(&format!(
            "Ok({}::query_with(transaction, row.get_opt::<usize, i32>(0).ok_or_else({}::no_result)??)?)",
            name, ERROR_TYPE,
        ));
    }

    fn impl_tupl_struct_query_fn(func: &mut Function, name: &str, rust: &RustType) {
        func.line("let statement = transaction.prepare_cached(Self::query_statement())?;");
        func.line("let rows = statement.query(&[&id])?;");
        func.line("let mut values = Vec::with_capacity(rows.len());");
        let inner = rust.clone().into_inner_type();
        if Self::is_sql_primitive(&inner) {
            let mut block = Block::new("for row in rows.iter()");
            let from_sql = inner.to_sql().to_rust();
            let load = format!(
                "row.get_opt::<usize, {}>(0).ok_or_else({}::no_result)??",
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
            "Ok({}::query_with(transaction, row.get_opt(0).ok_or_else({}::no_result)??)?)",
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
            )
            .arg("id", "i32")
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
            )
            .arg("row", &format!("&{}", ROW_TYPE))
            .ret(&format!("Result<Self, {}>", ERROR_TYPE))
    }

    pub fn is_sql_primitive(rust: &RustType) -> bool {
        match rust.clone().into_inner_type() {
            RustType::String => true,
            RustType::VecU8 => true,
            r => r.is_primitive(),
        }
    }

    fn struct_list_entry_insert_statement(struct_name: &str, field_name: &str) -> String {
        format!(
            "INSERT INTO {}(list, value) VALUES ($1, $2)",
            Model::<Sql>::struct_list_entry_table_name(struct_name, field_name),
        )
    }

    fn struct_list_entry_select_referenced_value_statement(
        struct_name: &str,
        field_name: &str,
        other_type: &str,
    ) -> String {
        let listentry_table = Model::<Sql>::struct_list_entry_table_name(struct_name, field_name);
        format!(
            "SELECT * FROM {} WHERE id IN (SELECT value FROM {} WHERE list = $1)",
            RustCodeGenerator::rust_variant_name(other_type),
            listentry_table,
        )
    }

    fn struct_list_entry_select_value_statement(struct_name: &str, field_name: &str) -> String {
        let listentry_table = Model::<Sql>::struct_list_entry_table_name(struct_name, field_name);
        format!("SELECT value FROM {} WHERE list = $1", listentry_table,)
    }

    fn list_entry_insert_statement(name: &str) -> String {
        format!("INSERT INTO {}ListEntry(list, value) VALUES ($1, $2)", name)
    }

    fn list_entry_query_statement(name: &str, inner: &RustType) -> String {
        if Self::is_sql_primitive(inner) {
            format!(
                "SELECT value FROM {}ListEntry WHERE {}ListEntry.list = $1",
                name, name
            )
        } else {
            let inner = inner.clone().into_inner_type().to_string();
            format!(
                "SELECT * FROM {} INNER JOIN {}ListEntry ON {}.id = {}ListEntry.value WHERE {}ListEntry.list = $1",
                inner, name, inner, name, name
            )
        }
    }
}
