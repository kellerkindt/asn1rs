use crate::gen::rust::shared_psql::*;
use crate::gen::rust::GeneratorSupplement;
use crate::gen::RustCodeGenerator;
use crate::model::rust::DataEnum;
use crate::model::rust::PlainEnum;
use crate::model::sql::{Sql, SqlType, ToSql};
use crate::model::{Definition, Model, Rust, RustType};
use codegen::{Block, Function, Impl, Scope};

const MODULE_NAME: &str = "apsql";
const FN_PREFIX: &str = "apsql_";

#[allow(clippy::module_name_repetitions)]
pub struct AsyncPsqlInserter;

impl GeneratorSupplement<Rust> for AsyncPsqlInserter {
    fn add_imports(&self, scope: &mut Scope) {
        scope.import("asn1rs::io", &format!("async_psql as {}", MODULE_NAME));
    }

    fn impl_supplement(&self, _scope: &mut Scope, _definition: &Definition<Rust>) {}

    fn extend_impl_of_struct(
        &self,
        name: &str,
        impl_scope: &mut Impl,
        fields: &[(String, RustType)],
    ) {
        AsyncPsqlInserter::append_retrieve_many_for_container_type(name, impl_scope);
        AsyncPsqlInserter::append_retrieve_for_container_type(name, impl_scope);
        AsyncPsqlInserter::append_load_struct(name, impl_scope, fields);

        let fn_insert = create_insert_fn(impl_scope, true);
        fn_insert.line(prepare_struct_insert_statement(name, fields));
        impl_insert_fn_content(false, true, name, fields, fn_insert);
    }

    fn extend_impl_of_enum(&self, _name: &str, impl_scope: &mut Impl, _r_enum: &PlainEnum) {
        AsyncPsqlInserter::append_retrieve_many_enums(impl_scope);
        AsyncPsqlInserter::append_retrieve_enum(impl_scope);

        create_load_fn(impl_scope, true).line(format!(
            "Self::{}(context, row.try_get::<usize, i32>(0)?).await",
            retrieve_fn_name()
        ));

        let fn_insert = create_insert_fn(impl_scope, false).arg_self();
        fn_insert.line("Ok(self.value_index() as i32)");
    }

    fn extend_impl_of_data_enum(&self, name: &str, impl_scope: &mut Impl, enumeration: &DataEnum) {
        Self::append_retrieve_many_for_container_type(name, impl_scope);
        Self::append_retrieve_for_container_type(name, impl_scope);

        let fn_load = create_load_fn(impl_scope, true);
        for (index, (variant, v_type)) in enumeration.variants().enumerate() {
            let mut block = Block::new(&format!(
                "if row.try_get::<usize, Option<i32>>({})?.is_some()",
                index + 1
            ));
            Self::append_load_field(false, name, &mut block, index, "value", v_type);
            block.line(&format!("return Ok({}::{}(value));", name, variant));
            fn_load.push_block(block);
        }
        fn_load.line(format!("Err({}::Error::RowUnloadable)", MODULE_NAME));

        let fn_insert = create_insert_fn(impl_scope, true);
        fn_insert.line(&format!(
            "let statement = context.prepared(\"{}\");",
            data_enum_insert_statement(name, enumeration)
        ));
        let mut updated_variants = Vec::with_capacity(enumeration.len());
        for (variant, v_type) in enumeration.variants() {
            let module_name = RustCodeGenerator::rust_module_name(variant);
            fn_insert.line(&format!(
                "let {} = if let Self::{}(value) = self {{ Some({}value) }} else {{ None }};",
                module_name,
                variant,
                if v_type.is_primitive() { "*" } else { "" }
            ));
            updated_variants.push((module_name, RustType::Option(Box::new(v_type.clone()))));
        }
        impl_insert_fn_content(false, false, name, &updated_variants[..], fn_insert);
    }

    fn extend_impl_of_tuple(&self, name: &str, impl_scope: &mut Impl, definition: &RustType) {
        let fields = [("0".to_string(), definition.clone())];

        // append_default_retrieve_many_fn(impl_scope);
        // let fn_retrieve = create_retrieve_fn(impl_scope);
        Self::append_retrieve_many_for_container_type(name, impl_scope);
        Self::append_retrieve_for_container_type(name, impl_scope);
        Self::append_load_tuple(name, impl_scope, definition);

        let fn_insert = create_insert_fn(impl_scope, true);
        fn_insert.line(&format!(
            "let statement = context.prepared(\"{}\");",
            tuple_struct_insert_statement(name)
        ));
        impl_insert_fn_content(true, true, name, &fields[..], fn_insert);
    }
}

fn impl_insert_fn_content(
    is_tuple_struct: bool,
    on_self: bool,
    name: &str,
    fields: &[(String, RustType)],
    container: &mut impl Container,
) {
    let mut params = Vec::default();
    let mut to_await = Vec::default();
    for insert in fields.iter().filter_map(|(field_name, r_type)| {
        let field_name = RustCodeGenerator::rust_field_name(field_name, true);
        let field_name_as_variable = if field_name
            .chars()
            .next()
            .map(|c| c.is_numeric())
            .unwrap_or(false)
        {
            Some(format!("value_{}", field_name))
        } else {
            None
        };
        let field_name_as_variable = field_name_as_variable.as_ref().map(String::as_str);

        if r_type.is_vec() {
            None
        } else {
            Some(insert_field(
                is_tuple_struct,
                on_self,
                name,
                container,
                &field_name,
                r_type,
                field_name_as_variable,
            ))
        }
    }) {
        match insert {
            FieldInsert::AsyncVec => {
                panic!("Unexpected result, vecs should not appear here because filtered");
            }
            FieldInsert::AsyncComplex(name) => {
                to_await.push(name.clone());
                params.push(name.clone());
            }
            FieldInsert::Primitive(name, _conversion) => {
                params.push(name);
            }
        }
    }
    if to_await.is_empty() {
        container.line("let statement = statement.await?;");
    } else {
        to_await.push("statement".to_string());
        let elements = to_await.join(", ");

        container.line(&format!(
            "let ({}) = {}::try_join!({})?;",
            elements, MODULE_NAME, elements
        ));
    }
    container.line(format!(
        "let id: i32 = context.query_one(&statement, &[{}]).await?.get(0);",
        params
            .iter()
            .map(|p| format!("&{}", p))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    to_await.clear();
    for insert in fields.iter().filter_map(|(field_name, r_type)| {
        if r_type.is_vec() {
            Some(insert_field(
                is_tuple_struct,
                on_self,
                name,
                container,
                &field_name,
                r_type,
                None,
            ))
        } else {
            None
        }
    }) {
        match insert {
            FieldInsert::AsyncVec => {} // fine
            FieldInsert::AsyncComplex(name) => {
                to_await.push(name);
            }
            _ => panic!("Unexpected result, only vecs should appear here because filtered"),
        }
    }
    if !to_await.is_empty() {
        container.line(&format!(
            "{}::try_join!({})?;",
            MODULE_NAME,
            to_await.join(", ")
        ));
    }
    container.line("Ok(id)");
}

fn prepare_struct_insert_statement(name: &str, fields: &[(String, RustType)]) -> String {
    format!(
        "let statement = context.prepared(\"{}\");",
        struct_insert_statement(name, fields)
    )
}

fn retrieve_many_fn_name() -> String {
    format!("{}retrieve_many", FN_PREFIX)
}

fn create_retrieve_many_fn(impl_scope: &mut Impl) -> &mut Function {
    impl_scope
        .new_fn(&retrieve_many_fn_name())
        .vis("pub async")
        .arg("context", format!("&{}::Context<'_>", MODULE_NAME))
        .arg("ids", "&[i32]")
        .ret(format!("Result<Vec<Self>, {}::Error>", MODULE_NAME))
}

fn retrieve_fn_name() -> String {
    format!("{}retrieve", FN_PREFIX)
}

fn create_retrieve_fn(impl_scope: &mut Impl, context_used: bool) -> &mut Function {
    impl_scope
        .new_fn(&retrieve_fn_name())
        .vis("pub async")
        .arg(
            if context_used { "context" } else { "_context" },
            format!("&{}::Context<'_>", MODULE_NAME),
        )
        .arg("id", "i32")
        .ret(format!("Result<Self, {}::Error>", MODULE_NAME))
}

fn load_fn_name() -> String {
    format!("{}load", FN_PREFIX)
}

fn create_load_fn(impl_scope: &mut Impl, context_used: bool) -> &mut Function {
    impl_scope
        .new_fn(&load_fn_name())
        .vis("pub async")
        .arg(
            if context_used { "context" } else { "_context" },
            format!("&{}::Context<'_>", MODULE_NAME),
        )
        .arg("row", format!("&{}::Row", MODULE_NAME))
        .ret(format!("Result<Self, {}::Error>", MODULE_NAME))
}

fn insert_fn_name() -> String {
    format!("{}insert", FN_PREFIX)
}

fn create_insert_fn(impl_scope: &mut Impl, context_used: bool) -> &mut Function {
    impl_scope
        .new_fn(&insert_fn_name())
        .arg_ref_self()
        .vis("pub async")
        .arg(
            if context_used { "context" } else { "_context" },
            format!("&{}::Context<'_>", MODULE_NAME),
        )
        .ret(format!("Result<i32, {}::PsqlError>", MODULE_NAME))
}

fn insert_field(
    is_tuple_struct: bool,
    on_self: bool,
    struct_name: &str,
    container: &mut impl Container,
    field_name: &str,
    r_type: &RustType,
    field_name_as_variable: Option<&str>,
) -> FieldInsert {
    if let RustType::Option(inner) = r_type {
        insert_optional_field(
            is_tuple_struct,
            on_self,
            struct_name,
            container,
            field_name,
            inner,
            field_name_as_variable,
        )
    } else if r_type.is_vec() {
        insert_vec_field(
            is_tuple_struct,
            on_self,
            struct_name,
            container,
            field_name,
            r_type,
        )
    } else if Model::<Sql>::is_primitive(r_type) {
        insert_sql_primitive_field(
            on_self,
            container,
            field_name,
            r_type,
            field_name_as_variable,
        )
    } else {
        insert_complex_field(on_self, container, field_name, field_name_as_variable)
    }
}

fn insert_optional_field(
    is_tuple_struct: bool,
    on_self: bool,
    struct_name: &str,
    container: &mut impl Container,
    field_name: &str,
    inner: &RustType,
    field_name_as_variable: Option<&str>,
) -> FieldInsert {
    insert_optional_field_maybe_async(
        is_tuple_struct,
        on_self,
        struct_name,
        container,
        field_name,
        inner,
        field_name_as_variable,
        false,
    )
}

fn insert_optional_field_maybe_async(
    is_tuple_struct: bool,
    on_self: bool,
    struct_name: &str,
    container: &mut impl Container,
    field_name: &str,
    inner: &RustType,
    field_name_as_variable: Option<&str>,
    call_await: bool,
) -> FieldInsert {
    let variable_name = field_name_as_variable.unwrap_or(field_name).to_string();
    let mut block_async = Block::new(&format!("let {} = async", field_name));
    if inner.as_no_option().is_vec() {
        let mut let_some = Block::new(&format!(
            "if let Some({}) = {}self.{}",
            variable_name,
            if inner.as_no_option().is_primitive() {
                ""
            } else {
                "&"
            },
            field_name
        ));
        if let RustType::Option(next) = inner {
            insert_optional_field_maybe_async(
                is_tuple_struct,
                on_self,
                struct_name,
                &mut let_some,
                &variable_name,
                next,
                None,
                true,
            );
        } else {
            // now is a vec
            insert_vec_field(
                is_tuple_struct,
                false,
                struct_name,
                &mut let_some,
                field_name,
                inner,
            );
        }
        let_some.line("Ok(())");
        let_some.after(" else { Ok(()) }");
        block_async.push_block(let_some);
    } else {
        let mut block_some = Block::new(&format!(
            "if let Some({}) = {}{}{}",
            variable_name,
            if inner.is_primitive() { "" } else { "&" },
            if on_self { "self." } else { "" },
            field_name
        ));
        let mut block_some_inner = Block::new("Ok(Some(");
        if Model::<Sql>::is_primitive(inner) {
            if inner.is_primitive() && inner.as_no_option().to_sql().to_rust().ne(inner) {
                let conversion = inner.as_no_option().to_sql().to_rust();
                block_some_inner.line(&format!(
                    "{} as {}",
                    variable_name,
                    conversion.to_inner_type_string()
                ));
            } else {
                block_some_inner.line(variable_name);
            }
        } else {
            match insert_field(
                is_tuple_struct,
                false,
                struct_name,
                &mut block_some_inner,
                &variable_name,
                inner,
                None,
            ) {
                FieldInsert::AsyncVec => {}
                FieldInsert::AsyncComplex(name) => {
                    block_some_inner.line(&format!("{}.await?", name));
                }
                FieldInsert::Primitive(name, _) => {
                    block_some_inner.line(name);
                }
            }
        }
        block_some_inner.after("))");
        block_some.push_block(block_some_inner);
        block_some.after(" else { Ok(None) } ");
        block_async.push_block(block_some);
    }
    if call_await {
        block_async.after(".await?;");
    } else {
        block_async.after(";");
    }
    container.push_block(block_async);
    FieldInsert::AsyncComplex(field_name.to_string())
}

fn insert_vec_field(
    is_tuple_struct: bool,
    on_self: bool,
    struct_name: &str,
    container: &mut impl Container,
    field_name: &str,
    r_type: &RustType,
) -> FieldInsert {
    let mut many_insert = Block::new("async");
    let inner_primitive = Model::<Sql>::is_primitive(r_type.as_inner_type());
    if inner_primitive {
        many_insert.line(&format!(
            "let inserted = &{}{};",
            if on_self { "self." } else { "" },
            field_name,
        ));
    } else {
        many_insert.line(&format!(
            "let inserted = {}::try_join_all({}{}.iter().map(|v| v.{}(context)));",
            MODULE_NAME,
            if on_self { "self." } else { "" },
            field_name,
            insert_fn_name()
        ));
    }
    many_insert.line(&format!(
        "let prepared = context.prepared(\"{}\");",
        if is_tuple_struct {
            list_entry_insert_statement(struct_name)
        } else {
            struct_list_entry_insert_statement(struct_name, field_name)
        }
    ));
    if inner_primitive {
        many_insert.line("let prepared = prepared.await?;");
    } else {
        many_insert.line(&format!(
            "let (inserted, prepared) = {}::try_join!(inserted, prepared)?;",
            MODULE_NAME
        ));
    }
    let conversion = r_type.to_sql().to_rust().ne(r_type);
    many_insert.line("let prepared = &prepared;");
    many_insert.line(&format!(
        "{}::try_join_all(inserted.iter().map(|i| async move {{ context.query(prepared, &[&id, {}]).await }} )).await",
        MODULE_NAME,
        if conversion { format!("&(*i as {})", r_type.to_sql().to_rust().to_inner_type_string()) } else { "&i".to_string() }
    ));
    many_insert.after(".await?;");
    container.push_block(many_insert);
    FieldInsert::AsyncVec
}

fn insert_sql_primitive_field(
    on_self: bool,
    container: &mut impl Container,
    field_name: &str,
    r_type: &RustType,
    field_name_as_variable: Option<&str>,
) -> FieldInsert {
    let rerust = r_type.to_sql().to_rust();
    let conversion = if rerust.ne(r_type) {
        Some(rerust)
    } else {
        None
    };
    let variable = field_name_as_variable.unwrap_or(field_name).to_string();
    container.line(&format!(
        "let {} = {}{}{}{}{};",
        variable,
        if r_type.is_primitive() { "" } else { "&" },
        if on_self { "self." } else { "" },
        field_name,
        conversion.as_ref().map(|_| " as ").unwrap_or_default(),
        conversion
            .as_ref()
            .map(|r| r.to_string())
            .unwrap_or_default(),
    ));
    FieldInsert::Primitive(variable, conversion)
}

fn insert_complex_field(
    on_self: bool,
    container: &mut impl Container,
    field_name: &str,
    field_name_as_variable: Option<&str>,
) -> FieldInsert {
    let variable_name = field_name_as_variable.unwrap_or(field_name).to_string();
    container.line(&format!(
        "let {} = {}{}.{}(context);",
        variable_name,
        if on_self { "self." } else { "" },
        field_name,
        insert_fn_name()
    ));
    FieldInsert::AsyncComplex(variable_name)
}

enum FieldInsert {
    AsyncVec,
    AsyncComplex(String),
    Primitive(String, Option<RustType>),
}

pub trait Container {
    fn line<T: ToString>(&mut self, line: T);
    fn push_block(&mut self, block: Block);
}

impl Container for Function {
    fn line<T: ToString>(&mut self, line: T) {
        Function::line(self, line);
    }

    fn push_block(&mut self, block: Block) {
        Function::push_block(self, block);
    }
}

impl Container for Block {
    fn line<T: ToString>(&mut self, line: T) {
        Block::line(self, line);
    }

    fn push_block(&mut self, block: Block) {
        Block::push_block(self, block);
    }
}

impl AsyncPsqlInserter {
    fn append_retrieve_many_enums(impl_scope: &mut Impl) {
        let fn_retrieve_many = create_retrieve_many_fn(impl_scope);
        fn_retrieve_many.line("let mut result = Vec::with_capacity(ids.len());");
        fn_retrieve_many.line(format!("for id in ids {{ result.push(Self::{}(context, *id).await?); }} // awaiting here is fine because {} returns immediately", retrieve_fn_name(), retrieve_fn_name()));
        fn_retrieve_many.line("Ok(result)");
    }

    fn append_retrieve_enum(impl_scope: &mut Impl) {
        create_retrieve_fn(impl_scope, false).line(format!(
            "Self::variant(id as usize).ok_or_else(|| {}::Error::UnexpectedVariant(id as usize))",
            MODULE_NAME,
        ));
    }

    fn append_retrieve_many_for_container_type(name: &str, impl_scope: &mut Impl) {
        let fn_retrieve_many = create_retrieve_many_fn(impl_scope);
        fn_retrieve_many.line(format!(
            "let prepared = context.prepared(\"{}\").await?;",
            select_statement_many(name)
        ));
        fn_retrieve_many.line("let rows = context.query(&prepared, &[&ids]).await?;");
        fn_retrieve_many.line(format!(
            "{}::try_join_all(rows.iter().map(|row| Self::{}(context, row))).await",
            MODULE_NAME,
            load_fn_name()
        ));
    }

    fn append_retrieve_for_container_type(name: &str, impl_scope: &mut Impl) {
        let fn_retrieve = create_retrieve_fn(impl_scope, true);
        fn_retrieve.line(format!(
            "let prepared = context.prepared(\"{}\").await?;",
            select_statement_single(name)
        ));
        fn_retrieve.line("let row = context.query_opt(&prepared, &[&id]).await?;");
        fn_retrieve.line(format!(
            "let row = row.ok_or_else(|| {}::Error::NoEntryFoundForId(id))?;",
            MODULE_NAME
        ));
        fn_retrieve.line(format!("Self::{}(context, &row).await", load_fn_name()));
    }

    fn append_load_struct(name: &str, impl_scope: &mut Impl, fields: &[(String, RustType)]) {
        let fn_load = create_load_fn(
            impl_scope,
            fields.iter().any(|(_name, f_type)| {
                !Model::<Sql>::is_primitive(f_type)
                    || Model::<Sql>::has_no_column_in_embedded_struct(f_type.as_no_option())
            }),
        );
        for (index, (field, f_type)) in fields.iter().enumerate() {
            AsyncPsqlInserter::append_load_field(false, name, fn_load, index, field, f_type);
        }

        let mut result_block = Block::new("Ok(Self");
        for (field, _type) in fields {
            result_block.line(format!(
                "{},",
                RustCodeGenerator::rust_field_name(field, true)
            ));
        }
        result_block.after(")");
        fn_load.push_block(result_block);
    }

    fn append_load_tuple(name: &str, impl_scope: &mut Impl, field_type: &RustType) {
        let fn_load = create_load_fn(impl_scope, true);
        AsyncPsqlInserter::append_load_field(true, name, fn_load, 0, "value", field_type);
        fn_load.line("Ok(Self(value))");
    }

    fn append_load_field(
        is_tuple_struct: bool,
        struct_name: &str,
        container: &mut impl Container,
        index: usize,
        field: &str,
        f_type: &RustType,
    ) {
        let sql = f_type.to_sql();
        if let RustType::Option(inner) = f_type {
            AsyncPsqlInserter::append_load_option_field(
                is_tuple_struct,
                struct_name,
                container,
                index,
                field,
                f_type,
                &sql,
                &**inner,
            )
        } else if let RustType::Vec(inner) = f_type {
            AsyncPsqlInserter::append_load_vec_field(
                is_tuple_struct,
                struct_name,
                container,
                field,
                f_type,
                &sql,
                &**inner,
            )
        } else if Model::<Sql>::is_primitive(f_type) {
            AsyncPsqlInserter::append_load_primitive_field(container, index, field, f_type, &sql);
        } else {
            container.line(format!(
                "let {} = row.try_get::<usize, i32>({})?;",
                RustCodeGenerator::rust_field_name(field, true),
                index + 1,
            ));
            Self::append_load_complex_field(container, field, f_type)
        }
    }

    fn append_load_primitive_field(
        container: &mut impl Container,
        index: usize,
        field: &str,
        f_type: &RustType,
        sql: &SqlType,
    ) {
        container.line(format!(
            "let {} = row.try_get::<usize, {}>({})?{};",
            RustCodeGenerator::rust_field_name(field, true),
            sql.to_rust().to_inner_type_string(),
            index + 1,
            if sql.to_rust().ne(f_type) {
                format!(" as {}", f_type.to_inner_type_string())
            } else {
                String::default()
            }
        ));
    }

    fn append_load_vec_field(
        is_tuple_struct: bool,
        struct_name: &str,
        container: &mut impl Container,
        field: &str,
        f_type: &RustType,
        sql: &SqlType,
        inner: &RustType,
    ) {
        if Model::<Sql>::is_primitive(inner) {
            container.line(format!(
                "let prepared = context.prepared(\"{}\").await?;",
                if is_tuple_struct {
                    list_entry_query_statement(struct_name, inner)
                } else {
                    struct_list_entry_select_value_statement(struct_name, field)
                }
            ));
            container.line(
                "let rows = context.query(&prepared, &[&row.try_get::<usize, i32>(0)?]).await?;",
            );
            container.line(format!(
                "let mut {} = Vec::with_capacity(rows.len());",
                RustCodeGenerator::rust_field_name(field, true)
            ));
            container.line(format!(
                "for row in rows {{ {}.push(row.try_get::<usize, {}>(0)?{}); }}",
                RustCodeGenerator::rust_field_name(field, true),
                inner.to_sql().to_rust().to_inner_type_string(),
                if sql.to_rust().ne(f_type) {
                    format!(" as {}", f_type.to_inner_type_string())
                } else {
                    String::default()
                }
            ));
        } else {
            container.line(format!(
                "let prepared = context.prepared(\"{}\").await?;",
                if is_tuple_struct {
                    list_entry_query_statement(struct_name, inner.as_inner_type())
                } else {
                    struct_list_entry_select_referenced_value_statement(
                        struct_name,
                        field,
                        &inner.to_inner_type_string(),
                    )
                }
            ));
            container.line(
                "let rows = context.query(&prepared, &[&row.try_get::<usize, i32>(0)?]).await?;",
            );
            container.line(format!(
                "let mut {} = Vec::with_capacity(rows.len());",
                RustCodeGenerator::rust_field_name(field, true)
            ));

            container.line(format!(
                "for row in rows {{ {}.push({}::{}(context, &row).await?); }}",
                RustCodeGenerator::rust_field_name(field, true),
                inner.to_inner_type_string(),
                load_fn_name(),
            ));
        }
    }

    #[allow(clippy::too_many_arguments)] // for now this is fine-ish
    fn append_load_option_field(
        is_tuple_struct: bool,
        struct_name: &str,
        container: &mut impl Container,
        index: usize,
        field: &str,
        f_type: &RustType,
        sql: &SqlType,
        inner: &RustType,
    ) {
        if inner.is_vec() {
            Self::append_load_field(is_tuple_struct, struct_name, container, index, field, inner);
            container.line(format!(
                "let {} = if {}.is_empty() {{ None }} else {{ Some({}) }};",
                RustCodeGenerator::rust_field_name(field, true),
                RustCodeGenerator::rust_field_name(field, true),
                RustCodeGenerator::rust_field_name(field, true),
            ))
        } else if Model::<Sql>::is_primitive(inner) {
            container.line(format!(
                "let {} = row.try_get::<usize, Option<{}>>({})?{};",
                RustCodeGenerator::rust_field_name(field, true),
                sql.to_rust().as_no_option().to_inner_type_string(),
                index + 1,
                if sql.to_rust().ne(f_type) {
                    format!(".map(|v| v as {})", inner.to_inner_type_string())
                } else {
                    String::default()
                }
            ));
        } else {
            let mut block = Block::new(&format!(
                "let {} = if let Some({}) = row.try_get::<usize, Option<i32>>({})?",
                RustCodeGenerator::rust_field_name(field, false),
                RustCodeGenerator::rust_field_name(field, false),
                index + 1
            ));
            Self::append_load_complex_field(&mut block, field, inner);
            block.line(format!(
                "Some({})",
                RustCodeGenerator::rust_field_name(field, false)
            ));
            block.after(" else { None };");
            container.push_block(block);
        }
    }

    fn append_load_complex_field(container: &mut impl Container, field: &str, f_type: &RustType) {
        container.line(format!(
            "let {} = {}::{}(context, {}).await?;",
            RustCodeGenerator::rust_field_name(field, true),
            f_type.to_inner_type_string(),
            retrieve_fn_name(),
            RustCodeGenerator::rust_field_name(field, true),
        ));
    }
}
