use crate::gen::rust::shared_psql::*;
use crate::gen::rust::GeneratorSupplement;
use crate::gen::rust::RustCodeGenerator;
use crate::model::rust::PlainEnum;
use crate::model::rust::{DataEnum, Field};
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

/// The tuple implementation has general flaws in where in which
/// it was initially designed to represent lists only. Later extensions
/// patch only certain uncovered edge-cases and are hacked into place.
///
/// The CHOICE/data-enum implementation also had/s quite a few flaws
#[allow(clippy::module_name_repetitions)]
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
        let implementation = Self::new_representable_impl(scope, name);
        Self::impl_table_name(Self::new_table_name_fn(implementation), name);
    }

    fn impl_insertable(scope: &mut Scope, Definition(name, rust): &Definition<Rust>) {
        let implementation = Self::new_insertable_impl(scope, name);
        match rust {
            Rust::Struct {
                fields,
                tag: _,
                extension_after: _,
                ordering: _,
            } => {
                Self::impl_struct_insert_statement(
                    Self::new_insert_statement_fn(implementation),
                    name,
                    fields,
                );
                Self::impl_struct_insert_fn(
                    Self::new_insert_fn(implementation, true),
                    name,
                    fields.iter().map(Field::fallback_representation),
                );
            }
            Rust::DataEnum(enumeration) => {
                Self::impl_data_enum_insert_statement(
                    Self::new_insert_statement_fn(implementation),
                    name,
                    enumeration,
                );
                Self::impl_data_enum_insert_fn(
                    Self::new_insert_fn(implementation, true),
                    name,
                    enumeration,
                );
            }
            Rust::Enum(_) => {
                Self::impl_enum_insert_statement(Self::new_insert_statement_fn(implementation));
                Self::impl_enum_insert_fn(Self::new_insert_fn(implementation, false));
            }
            Rust::TupleStruct { r#type: rust, .. } => {
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

    fn impl_struct_insert_statement(function: &mut Function, name: &str, fields: &[Field]) {
        if fields.is_empty() {
            Self::impl_tuple_insert_statement(function, name);
        } else {
            function.line(&format!("\"{}\"", struct_insert_statement(name, fields)));
        }
    }

    fn impl_data_enum_insert_statement(
        function: &mut Function,
        name: &str,
        enumeration: &DataEnum,
    ) {
        if enumeration.is_empty() {
            Self::impl_tuple_insert_statement(function, name);
        } else {
            function.line(&format!(
                "\"{}\"",
                data_enum_insert_statement(name, enumeration)
            ));
        }
    }

    fn impl_enum_insert_statement(function: &mut Function) {
        function.line("\"\"");
    }

    fn impl_tuple_insert_statement(function: &mut Function, name: &str) {
        function.line(&format!("\"{}\"", tuple_struct_insert_statement(name)));
    }

    fn impl_struct_insert_fn<'a>(
        function: &mut Function,
        struct_name: &str,
        fields: impl ExactSizeIterator<Item = &'a (String, RustType)>,
    ) {
        let mut variables = Vec::with_capacity(fields.len());
        let mut vecs = Vec::new();
        for (name, rust) in fields {
            let name = RustCodeGenerator::rust_field_name(name, true);
            let sql_primitive = Model::<Sql>::is_primitive(rust);
            let is_vec = rust.is_vec();

            if is_vec {
                vecs.push((name.clone(), rust.clone()));
                continue;
            } else {
                variables.push(format!("&{}", name));
            }
            if sql_primitive {
                function.line(&format!(
                    "let {} = {}self.{};",
                    name,
                    if rust.is_primitive() { "" } else { "&" },
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
                    &struct_list_entry_insert_statement(struct_name, &name),
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
        if inner_sql.to_rust().into_inner_type().similar(&inner_rust) {
            None
        } else {
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
        }
    }

    fn wrap_for_query_in_as_or_from_if_required(name: &str, rust: &RustType) -> Option<String> {
        let inner_sql = rust.clone().into_inner_type().to_sql();
        let inner_rust = rust.clone().into_inner_type();
        if inner_sql.to_rust().into_inner_type().similar(&inner_rust) {
            None
        } else {
            Some({
                let as_target = inner_rust.to_string();
                if let RustType::Option(_) = rust {
                    format!("{}.map(|v| v as {})", name, as_target)
                } else {
                    format!("{} as {}", name, as_target)
                }
            })
        }
    }

    fn impl_data_enum_insert_fn(function: &mut Function, name: &str, enumeration: &DataEnum) {
        let mut variables = Vec::with_capacity(enumeration.len());
        for variant in enumeration.variants() {
            let variable = RustCodeGenerator::rust_field_name(
                &RustCodeGenerator::rust_module_name(variant.name()),
                true,
            );
            let sql_primitive = Model::<Sql>::is_primitive(variant.r#type());
            variables.push(format!("&{}", variable));
            let mut block_if = Block::new(&format!(
                "let {} = if let {}::{}(value) = self",
                variable,
                name,
                variant.name()
            ));

            if sql_primitive {
                block_if.line(format!(
                    "Some({})",
                    Self::wrap_for_insert_in_as_or_from_if_required("*value", variant.r#type())
                        .unwrap_or_else(|| "value".to_string())
                ));
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
            list_entry_insert_statement(name)
        ));
        function.push_block(Self::list_insert_for_each("0", rust, "list"));
        function.line("Ok(list)");
    }

    /// Expects a variable called `statement` to be reachable and usable
    fn list_insert_for_each(name: &str, rust: &RustType, list: &str) -> Block {
        let mut block_for = if rust.as_no_option().is_vec() {
            Block::new(&if let RustType::Option(_) = rust {
                format!("for value in self.{}.iter().flatten()", name)
            } else {
                format!("for value in &self.{}", name)
            })
        } else if rust.is_option() {
            Block::new(&format!("if let Some(value) = &self.{}", name))
        } else {
            let mut block = Block::new("");
            block.line(format!("let value = &self.{};", name));
            block
        };
        if Model::<Sql>::is_primitive(rust) {
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
                        format!("(*value) as {}", as_target)
                    },
                ));
            }
        } else {
            block_for.line("let value = value.insert_with(transaction)?;");
        }
        block_for.line(format!("statement.execute(&[&{}, &value])?;", list));
        block_for
    }

    fn impl_queryable(scope: &mut Scope, Definition(name, rust): &Definition<Rust>) {
        let implementation = Self::new_queryable_impl(scope, name);
        match rust {
            Rust::Struct {
                fields,
                tag: _,
                extension_after: _,
                ordering: _,
            } => {
                Self::impl_query_statement(Self::new_query_statement_fn(implementation), name);
                Self::impl_struct_query_fn(Self::new_query_fn(implementation, true), name);
                Self::impl_struct_load_fn(
                    Self::new_load_fn(
                        implementation,
                        fields.iter().any(|field| {
                            !Model::<Sql>::is_primitive(field.r#type()) || field.r#type().is_vec()
                        }),
                    ),
                    name,
                    fields.iter().map(Field::fallback_representation),
                );
            }
            Rust::DataEnum(enumeration) => {
                Self::impl_query_statement(Self::new_query_statement_fn(implementation), name);
                Self::impl_data_enum_query_fn(Self::new_query_fn(implementation, true), name);
                Self::impl_data_enum_load_fn(
                    Self::new_load_fn(implementation, true),
                    name,
                    enumeration,
                );
            }
            Rust::Enum(r_enum) => {
                Self::impl_empty_query_statement(Self::new_query_statement_fn(implementation));
                Self::impl_enum_query_fn(Self::new_query_fn(implementation, false), name, r_enum);
                Self::impl_enum_load_fn(Self::new_load_fn(implementation, true), name);
            }
            Rust::TupleStruct { r#type: rust, .. } => {
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
        func.line(&format!("\"{}\"", select_statement_single(name)));
    }

    fn impl_empty_query_statement(func: &mut Function) {
        func.line("\"\"");
    }

    fn impl_tupl_query_statement(func: &mut Function, name: &str, inner: &RustType) {
        func.line(&format!("\"{}\"", list_entry_query_statement(name, inner)));
    }

    fn impl_struct_query_fn(func: &mut Function, name: &str) {
        func.line("let statement = transaction.prepare_cached(Self::query_statement())?;");
        func.line("let rows = statement.query(&[&id])?;");
        func.line(&format!(
            "Ok({}::load_from(transaction, &rows.iter().next().ok_or_else({}::no_result)?)?)",
            name, ERROR_TYPE,
        ));
    }
    fn impl_struct_load_fn<'a>(
        func: &mut Function,
        struct_name: &str,
        variants: impl ExactSizeIterator<Item = &'a (String, RustType)>,
    ) {
        let mut block = Block::new(&format!("Ok({}", struct_name));
        let mut index_negative_offset = 0;

        for (index, (name, rust)) in variants.enumerate() {
            let index = index - index_negative_offset;

            // Lists do not have a entry in the `holding` table but
            // a third table referencing both (M:N relation), therefore
            // the index must not be incremented, otherwise one column
            // in the container table would be skipped
            if Model::<Sql>::has_no_column_in_embedded_struct(rust) {
                index_negative_offset += 1;
            }

            if rust.is_vec() {
                let mut load_block = Block::new(&format!(
                    "{}:",
                    RustCodeGenerator::rust_field_name(name, true)
                ));

                load_block.line("let mut vec = Vec::default();");

                load_block.line(&format!(
                        "let rows = transaction.prepare_cached(\"{}\")?.query(&[&row.get_opt::<usize, i32>(0).ok_or_else(PsqlError::no_result)??])?;",

                        if let RustType::Complex(complex, _tag) = rust.clone().into_inner_type() {
                            struct_list_entry_select_referenced_value_statement(
                                struct_name,
                                name,
                                &complex
                            )
                        } else {
                            struct_list_entry_select_value_statement(
                                struct_name, name
                            )
                        }
                    ));
                let mut rows_foreach = Block::new("for row in rows.iter()");
                if let RustType::Complex(complex, _tag) = rust.clone().into_inner_type() {
                    rows_foreach.line(&format!(
                        "vec.push({}::load_from(transaction, &row)?);",
                        complex
                    ));
                } else {
                    let rust = rust.clone().into_inner_type();
                    let sql = rust.to_sql();
                    rows_foreach.line(&format!(
                        "let value = row.get_opt::<usize, {}>(0).ok_or_else({}::no_result)??;",
                        sql.to_rust().to_string(),
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
            } else if Model::<Sql>::is_primitive(rust) {
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
    fn impl_data_enum_load_fn(func: &mut Function, name: &str, enumeration: &DataEnum) {
        func.line(format!(
            "let index = {}::first_present(row, &[{}])?;",
            ERROR_TYPE,
            enumeration
                .variants()
                .enumerate()
                .map(|e| format!("{}", e.0 + 1))
                .collect::<Vec<String>>()
                .join(", ")
        ));
        let mut block = Block::new("match index");
        for (index, variant) in enumeration.variants().enumerate() {
            let mut block_case = Block::new(&format!(
                "{} => Ok({}::{}(",
                index + 1,
                name,
                variant.name()
            ));

            if Model::<Sql>::is_primitive(variant.r#type().as_inner_type()) {
                if let Some(wrap) =
                    Self::wrap_for_query_in_as_or_from_if_required("", variant.r#type())
                {
                    block_case.line(format!(
                        "row.get_opt::<_, {}>({}).ok_or_else({}::no_result)??{}",
                        variant
                            .r#type()
                            .as_inner_type()
                            .to_sql()
                            .to_rust()
                            .to_string(),
                        index + 1,
                        ERROR_TYPE,
                        wrap
                    ));
                } else {
                    block_case.line(format!(
                        "row.get_opt::<_, {}>({}).ok_or_else({}::no_result)??",
                        variant
                            .r#type()
                            .as_inner_type()
                            .to_sql()
                            .to_rust()
                            .to_string(),
                        index + 1,
                        ERROR_TYPE
                    ));
                }
            } else {
                block_case.line(&format!(
                    "{}::query_with(transaction, row.get({}))?",
                    variant.r#type().clone().into_inner_type().to_string(),
                    index + 1
                ));
            }

            block_case.after(")),");
            block.push_block(block_case);
        }
        block.line(&format!("_ => Err({}::no_result()),", ERROR_TYPE));
        func.push_block(block);
    }

    fn impl_enum_query_fn(func: &mut Function, name: &str, r_enum: &PlainEnum) {
        let mut block = Block::new("match id");
        for (index, variant) in r_enum.variants().enumerate() {
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
        let inner = rust.clone().into_inner_type();
        if Model::<Sql>::is_primitive(&inner) {
            let from_sql = inner.to_sql().to_rust();

            let load = format!(
                "row.get_opt::<usize, {}>(0).ok_or_else({}::no_result)??",
                from_sql.to_string(),
                ERROR_TYPE,
            );
            let load_wrapped =
                Self::wrap_for_query_in_as_or_from_if_required(&load, rust).unwrap_or(load);

            if rust.is_vec() {
                func.line("let mut values = Vec::with_capacity(rows.len());");
                let mut block = Block::new("for row in rows.iter()");
                block.line(&format!("values.push({});", load_wrapped));
                func.push_block(block);
            } else {
                func.line(format!(
                    "let row = rows.iter().next().ok_or_else({}::no_result)?;",
                    ERROR_TYPE
                ));
                func.line(format!("let values = {};", load_wrapped));
            }
        } else if rust.is_vec() {
            func.line("let mut values = Vec::with_capacity(rows.len());");
            let mut block = Block::new("for row in rows.iter()");
            block.line(&format!(
                "values.push({}::load_from(transaction, &row)?);",
                inner.to_string()
            ));
            func.push_block(block);
        } else {
            func.line(format!(
                "let row = rows.iter().next().ok_or_else({}::no_result)?;",
                ERROR_TYPE
            ));
            func.line(format!(
                "let values = {}::load_from(transaction, &row)?;",
                inner.to_string()
            ));
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
}
