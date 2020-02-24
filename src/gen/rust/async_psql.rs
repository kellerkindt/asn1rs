use crate::gen::rust::psql::PsqlInserter;
use crate::gen::rust::GeneratorSupplement;
use crate::gen::RustCodeGenerator;
use crate::model::sql::{Sql, ToSql};
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

        let fn_insert = create_insert_fn(impl_scope);
        fn_insert.line(prepare_struct_insert_statement(name, fields));
        impl_insert_fn_content(true, name, fields, fn_insert);
    }

    fn extend_impl_of_enum(&self, name: &str, impl_scope: &mut Impl, _variants: &[String]) {
        AsyncPsqlInserter::append_retrieve_many_enums(impl_scope);

        create_load_fn(impl_scope).line(format!(
            "Self::{}(context, row.try_get::<usize, i32>(0)?).await",
            retrieve_fn_name()
        ));

        let fn_insert = create_insert_fn(impl_scope);
        fn_insert.line("Ok(self.value_index() as i32)");
    }

    fn extend_impl_of_data_enum(
        &self,
        name: &str,
        impl_scope: &mut Impl,
        variants: &[(String, RustType)],
    ) {
        Self::append_retrieve_many_for_container_type(name, impl_scope);
        Self::append_retrieve_for_container_type(name, impl_scope);

        let fn_load = create_load_fn(impl_scope);
        for (index, (variant, v_type)) in variants.iter().enumerate() {
            let mut block = Block::new(&format!(
                "if row.try_get::<usize, Option<i32>>({})?.is_some()",
                index + 1
            ));
            Self::append_load_field(name, &mut block, index, "value", v_type);
            block.line(&format!("return Ok({}::{}(value));", name, variant));
            fn_load.push_block(block);
        }
        fn_load.line(format!("Err({}::Error::RowUnloadable)", MODULE_NAME));

        let fn_insert = create_insert_fn(impl_scope);
        fn_insert.line(&format!(
            "let statement = context.prepared(\"INSERT INTO {}({}) VALUES({}) RETURNING id\");",
            name,
            variants
                .iter()
                .map(|(name, _)| RustCodeGenerator::rust_module_name(name))
                .collect::<Vec<_>>()
                .join(", "),
            variants
                .iter()
                .enumerate()
                .map(|(num, _)| format!("${}", num + 1))
                .collect::<Vec<_>>()
                .join(", "),
        ));
        let mut updated_variants = Vec::with_capacity(variants.len());
        for (variant, v_type) in variants {
            let module_name = RustCodeGenerator::rust_module_name(variant);
            fn_insert.line(&format!(
                "let {} = if let Self::{}(value) = self {{ Some(value) }} else {{ None }};",
                module_name, variant
            ));
            updated_variants.push((module_name, RustType::Option(Box::new(v_type.clone()))));
        }
        impl_insert_fn_content(false, name, &updated_variants[..], fn_insert);
    }

    fn extend_impl_of_tuple(&self, name: &str, impl_scope: &mut Impl, definition: &RustType) {
        append_default_retrieve_many_fn(impl_scope);
        let fn_retrieve = create_retrieve_fn(impl_scope);
        let fn_load = create_load_fn(impl_scope);

        let fields = [("0".to_string(), definition.clone())];
        let fn_insert = create_insert_fn(impl_scope);
        fn_insert.line(&format!(
            "let statement = context.prepared(\"INSERT INTO {} DEFAULT VALUES RETURNING id\");",
            name
        ));
        impl_insert_fn_content(true, name, &fields[..], fn_insert);
    }
}

#[deprecated]
fn append_default_retrieve_many_fn(impl_scope: &mut Impl) {
    create_retrieve_many_fn(impl_scope).line(format!(
        "{}::try_join_all(ids.iter().map(|id| Self::{}(context, *id))).await",
        MODULE_NAME,
        retrieve_fn_name()
    ));
}

fn impl_insert_fn_content(
    on_self: bool,
    name: &str,
    fields: &[(String, RustType)],
    container: &mut impl Container,
) {
    let mut primitives = Vec::default();
    let mut params = Vec::default();
    let mut to_await = Vec::default();
    for insert in fields
        .iter()
        .filter(|(_field_name, r_type)| !r_type.is_vec())
        .map(|(field_name, r_type)| {
            insert_field(
                on_self,
                name,
                container,
                &RustCodeGenerator::rust_field_name(field_name, true),
                r_type,
            )
        })
    {
        match insert {
            FieldInsert::AsyncVec => {
                panic!("Unexpected result, vecs should not appear here because filtered");
            }
            FieldInsert::AsyncComplex(name) => {
                to_await.push(name.clone());
                params.push(name.clone());
            }
            FieldInsert::Primitive(name, conversion) => {
                primitives.push((name.clone(), conversion));
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
        "let id: i32 = context.transaction().query_one(&statement, &[{}]).await?.get(0);",
        params
            .iter()
            .map(|p| format!("&{}", p))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    to_await.clear();
    for insert in fields
        .iter()
        .filter(|(_field_name, r_type)| r_type.is_vec())
        .map(|(field_name, r_type)| {
            insert_field(
                on_self,
                name,
                container,
                &RustCodeGenerator::rust_field_name(field_name, true),
                r_type,
            )
        })
    {
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
    let fields = fields
        .iter()
        .filter_map(|(name, field)| {
            if field.is_vec() {
                None
            } else {
                Some(Model::sql_column_name(name))
            }
        })
        .collect::<Vec<_>>();

    let mut line = "let statement = context.prepared(\"INSERT INTO ".to_string();
    line.push_str(name);
    line.push_str("(");
    line.push_str(&fields.join(", "));
    line.push_str(") VALUES(");
    line.push_str(
        &(0..fields.len())
            .map(|i| format!("${}", i + 1))
            .collect::<Vec<_>>()
            .join(", "),
    );
    line.push_str(") RETURNING id\");");
    line
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

fn create_retrieve_fn(impl_scope: &mut Impl) -> &mut Function {
    impl_scope
        .new_fn(&retrieve_fn_name())
        .vis("pub async")
        .arg("context", format!("&{}::Context<'_>", MODULE_NAME))
        .arg("id", "i32")
        .ret(format!("Result<Self, {}::Error>", MODULE_NAME))
}

fn load_fn_name() -> String {
    format!("{}load", FN_PREFIX)
}

fn create_load_fn(impl_scope: &mut Impl) -> &mut Function {
    impl_scope
        .new_fn(&load_fn_name())
        .vis("pub async")
        .arg("context", format!("&{}::Context<'_>", MODULE_NAME))
        .arg("row", format!("&{}::Row", MODULE_NAME))
        .ret(format!("Result<Self, {}::Error>", MODULE_NAME))
}

fn insert_fn_name() -> String {
    format!("{}insert", FN_PREFIX)
}

fn create_insert_fn(impl_scope: &mut Impl) -> &mut Function {
    impl_scope
        .new_fn(&insert_fn_name())
        .arg_ref_self()
        .vis("pub async")
        .arg("context", format!("&{}::Context<'_>", MODULE_NAME))
        .ret(format!("Result<i32, {}::PsqlError>", MODULE_NAME))
}

fn insert_field(
    on_self: bool,
    struct_name: &str,
    container: &mut impl Container,
    field_name: &str,
    r_type: &RustType,
) -> FieldInsert {
    if let RustType::Option(inner) = r_type {
        insert_optional_field(on_self, struct_name, container, field_name, inner)
    } else if r_type.is_vec() {
        insert_vec_field(on_self, struct_name, container, field_name, r_type)
    } else if Model::<Sql>::is_primitive(r_type) {
        insert_sql_primitive_field(on_self, container, field_name, r_type)
    } else {
        insert_complex_field(on_self, container, field_name)
    }
}

fn insert_optional_field(
    on_self: bool,
    struct_name: &str,
    container: &mut impl Container,
    field_name: &str,
    inner: &RustType,
) -> FieldInsert {
    insert_optional_field_maybe_async(on_self, struct_name, container, field_name, inner, false)
}

fn insert_optional_field_maybe_async(
    on_self: bool,
    struct_name: &str,
    container: &mut impl Container,
    field_name: &str,
    inner: &RustType,
    call_await: bool,
) -> FieldInsert {
    let mut block_async = Block::new(&format!("let {} = async", field_name));
    if inner.as_no_option().is_vec() {
        let mut let_some = Block::new(&format!(
            "if let Some({}) = {}self.{}",
            field_name,
            if inner.as_no_option().is_primitive() {
                ""
            } else {
                "&"
            },
            field_name
        ));
        if let RustType::Option(next) = inner {
            insert_optional_field_maybe_async(
                on_self,
                struct_name,
                &mut let_some,
                field_name,
                &next,
                true,
            );
        } else {
            // now is a vec
            insert_vec_field(false, struct_name, &mut let_some, field_name, inner);
        }
        let_some.line("Ok(())");
        let_some.after(" else { Ok(()) }");
        block_async.push_block(let_some);
    } else {
        let mut block_some = Block::new(&format!(
            "if let Some({}) = {}{}{}",
            field_name,
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
                    field_name,
                    conversion.to_inner_type_string()
                ));
            } else {
                block_some_inner.line(field_name);
            }
        } else {
            match insert_field(false, struct_name, &mut block_some_inner, field_name, inner) {
                FieldInsert::AsyncVec => {}
                FieldInsert::AsyncComplex(name) => {
                    block_some_inner.line(&format!("{}.await?", name));
                }
                FieldInsert::Primitive(name, _) => {
                    block_some_inner.line(&format!("{}", name));
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
        PsqlInserter::struct_list_entry_insert_statement(struct_name, field_name)
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
        "{}::try_join_all(inserted.into_iter().map(|i| async move {{ context.transaction().query(prepared, &[&id, {}]).await }} )).await",
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
) -> FieldInsert {
    let rerust = r_type.to_sql().to_rust();
    let conversion = if rerust.ne(r_type) {
        Some(rerust)
    } else {
        None
    };
    container.line(&format!(
        "let {} = {}{}{}{}{};",
        field_name,
        if r_type.is_primitive() { "" } else { "&" },
        if on_self { "self." } else { "" },
        field_name,
        conversion.as_ref().map(|_| " as ").unwrap_or_default(),
        conversion
            .as_ref()
            .map(|r| r.to_string())
            .unwrap_or_default(),
    ));
    FieldInsert::Primitive(field_name.to_string(), conversion)
}

fn insert_complex_field(
    on_self: bool,
    container: &mut impl Container,
    field_name: &str,
) -> FieldInsert {
    container.line(&format!(
        "let {} = {}{}.{}(&context);",
        field_name,
        if on_self { "self." } else { "" },
        field_name,
        insert_fn_name()
    ));
    FieldInsert::AsyncComplex(field_name.to_string())
}

enum FieldInsert {
    AsyncVec,
    AsyncComplex(String),
    Primitive(String, Option<RustType>),
}

impl FieldInsert {
    pub fn is_async(&self) -> bool {
        match self {
            FieldInsert::AsyncVec => true,
            FieldInsert::AsyncComplex(_) => true,
            FieldInsert::Primitive(_, _) => false,
        }
    }

    pub fn is_vec(&self) -> bool {
        if let FieldInsert::AsyncVec = self {
            true
        } else {
            false
        }
    }

    pub fn is_complex(&self) -> bool {
        if let FieldInsert::AsyncComplex(_) = &self {
            true
        } else {
            false
        }
    }

    pub fn is_primitve(&self) -> bool {
        if let FieldInsert::Primitive(_, _) = &self {
            true
        } else {
            false
        }
    }
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
        fn_retrieve_many.line(format!("let mut result = Vec::with_capacity(ids.len());"));
        fn_retrieve_many.line(format!("for id in ids {{ result.push(Self::{}(context, *id).await?); }} // awaiting here is fine because {} returns immediately", retrieve_fn_name(), retrieve_fn_name()));
        fn_retrieve_many.line("Ok(result)");
        create_retrieve_fn(impl_scope).line(format!(
            "Self::variant(id as usize).ok_or_else(|| {}::Error::UnexpectedVariant(id as usize))",
            MODULE_NAME,
        ));
    }
}

impl AsyncPsqlInserter {
    fn append_retrieve_many_for_container_type(name: &str, impl_scope: &mut Impl) {
        let fn_retrieve_many = create_retrieve_many_fn(impl_scope);
        fn_retrieve_many.line(format!(
            "let prepared = context.prepared(\"SELECT * FROM {} WHERE id = ANY($1)\").await?;",
            name
        ));
        fn_retrieve_many.line("let rows = context.transaction().query(&prepared, &[&ids]).await?;");
        fn_retrieve_many.line(format!(
            "{}::try_join_all(rows.iter().map(|row| Self::{}(context, row))).await",
            MODULE_NAME,
            load_fn_name()
        ));
    }

    fn append_retrieve_for_container_type(name: &str, impl_scope: &mut Impl) {
        let fn_retrieve = create_retrieve_fn(impl_scope);
        fn_retrieve.line(format!(
            "let prepared = context.prepared(\"SELECT * FROM {} WHERE id = $1\").await?;",
            name
        ));
        fn_retrieve.line("let row = context.transaction().query_opt(&prepared, &[&id]).await?;");
        fn_retrieve.line(format!(
            "let row = row.ok_or_else(|| {}::Error::NoEntryFoundForId(id))?;",
            MODULE_NAME
        ));
        fn_retrieve.line(format!("Self::{}(context, &row).await", load_fn_name()));
    }

    fn append_load_struct(name: &str, impl_scope: &mut Impl, fields: &[(String, RustType)]) {
        let fn_load = create_load_fn(impl_scope);
        for (index, (field, f_type)) in fields.iter().enumerate() {
            AsyncPsqlInserter::append_load_field(name, fn_load, index, field, f_type);
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

    fn append_load_field(
        struct_name: &str,
        container: &mut impl Container,
        index: usize,
        field: &str,
        f_type: &RustType,
    ) -> () {
        let sql = f_type.to_sql();
        if let RustType::Option(inner) = f_type {
            if inner.is_vec() {
                Self::append_load_field(struct_name, container, index, field, &**inner);
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
                    "let {} = if let Some({}_id) = row.try_get::<usize, Option<i32>>({})?",
                    RustCodeGenerator::rust_field_name(field, false),
                    RustCodeGenerator::rust_field_name(field, false),
                    index + 1
                ));
                Self::append_load_complex_field(struct_name, &mut block, index, field, &*inner);
                block.line(format!(
                    "Some({})",
                    RustCodeGenerator::rust_field_name(field, false)
                ));
                block.after(" else { None };");
                container.push_block(block);
            }
        } else if let RustType::Vec(inner) = f_type {
            if Model::<Sql>::is_primitive(inner) {
                container.line(format!(
                    "let prepared = context.prepared(\"{}\").await?;",
                    PsqlInserter::struct_list_entry_select_value_statement(struct_name, field)
                ));
                container.line("let rows = context.transaction().query(&prepared, &[&row.try_get::<usize, i32>(0)?]).await?;");
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
                    PsqlInserter::struct_list_entry_select_referenced_value_statement(
                        struct_name,
                        field,
                        &inner.to_inner_type_string()
                    )
                ));
                container.line("let rows = context.transaction().query(&prepared, &[&row.try_get::<usize, i32>(0)?]).await?;");
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
        } else if Model::<Sql>::is_primitive(f_type) {
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
        } else {
            container.line(format!(
                "let {}_id = row.try_get::<usize, i32>({})?;",
                RustCodeGenerator::rust_field_name(field, true),
                index + 1,
            ));
            Self::append_load_complex_field(struct_name, container, index, field, f_type)
        }
    }
    fn append_load_complex_field(
        struct_name: &str,
        container: &mut impl Container,
        index: usize,
        field: &str,
        f_type: &RustType,
    ) {
        container.line(format!(
            "let {} = {}::{}(context, {}_id).await?;",
            RustCodeGenerator::rust_field_name(field, true),
            f_type.to_inner_type_string(),
            retrieve_fn_name(),
            RustCodeGenerator::rust_field_name(field, true),
        ));
    }
}
