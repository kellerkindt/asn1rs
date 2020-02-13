use crate::gen::rust::psql::PsqlInserter;
use crate::gen::rust::GeneratorSupplement;
use crate::gen::RustCodeGenerator;
use crate::model::sql::{Sql, ToSql};
use crate::model::{Definition, Model, Rust, RustType};
use codegen::{Block, Function, Impl, Scope};

const MODULE_NAME: &str = "apsql";
const FN_PREFIX: &str = "psql_";

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
        let fun = create_insert_fn(impl_scope);
        fun.line(prepare_struct_insert_statement(name, fields));

        let mut primitives = Vec::default();
        let mut params = Vec::default();
        let mut to_await = Vec::default();

        for insert in fields
            .iter()
            .filter(|(_field_name, r_type)| !r_type.is_vec())
            .map(|(field_name, r_type)| {
                insert_field(
                    true,
                    name,
                    fun,
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
            fun.line("let statement = statement.await?;");
        } else {
            to_await.push("statement".to_string());
            let elements = to_await.join(", ");

            fun.line(&format!(
                "let ({}) = {}::try_join!({})?;",
                elements, MODULE_NAME, elements
            ));
        }

        fun.line(format!(
            "let id: i32 = context.transaction().query_one(&statement, &[{}]).await?.get(1);",
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
                    true,
                    name,
                    fun,
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
            fun.line(&format!(
                "{}::try_join!({})?;",
                MODULE_NAME,
                to_await.join(", ")
            ));
        }
        fun.line("Ok(id)");
    }

    fn extend_impl_of_enum(&self, name: &str, impl_scope: &mut Impl, _variants: &[String]) {
        let fun = create_insert_fn(impl_scope);
        fun.line("Ok(self.value_index() as i32)");
    }

    fn extend_impl_of_data_enum(
        &self,
        name: &str,
        _impl_scope: &mut Impl,
        _variants: &[(String, RustType)],
    ) {
    }

    fn extend_impl_of_tuple(&self, name: &str, _impl_scope: &mut Impl, _definition: &RustType) {}
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

fn insert_fn_name() -> String {
    format!("{}insert", FN_PREFIX)
}

fn create_insert_fn(impl_scope: &mut Impl) -> &mut Function {
    impl_scope
        .new_fn(&insert_fn_name())
        .arg_ref_self()
        .vis("pub async")
        .arg("context", format!("&{}::Context<'_>", MODULE_NAME))
        .ret(format!("Result<i32, {}::Error>", MODULE_NAME))
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
    let mut block_async = Block::new(&format!("let {} = async", field_name,));
    let mut block_some = Block::new(&format!(
        "if let Some({}) = {}{}{}",
        field_name,
        if inner.is_primitive() { "" } else { "&" },
        if on_self { "self." } else { "" },
        field_name
    ));
    let mut block_some_inner = Block::new("Ok(Some(");
    match insert_field(false, struct_name, &mut block_some_inner, field_name, inner) {
        FieldInsert::AsyncVec => {}
        FieldInsert::AsyncComplex(name) => {
            block_some_inner.line(&format!("{}.await?", name));
        }
        FieldInsert::Primitive(name, _) => {
            block_some_inner.line(&format!("{}", name));
        }
    }
    block_some_inner.after("))");
    block_some.push_block(block_some_inner);
    block_some.after("else { Ok(None) } ");
    block_async.push_block(block_some);
    block_async.after(";");
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
    many_insert.line(&format!(
        "{}::try_join_all(inserted.iter().map(|i| context.transaction().query(&prepared, &[&id, {}]))).await?;",
        MODULE_NAME,
        if conversion { format!("&(*i as {})", r_type.to_sql().to_rust().to_inner_type_string()) } else { "i".to_string() }
    ));
    many_insert.line("Ok(())");
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
    fn line(&mut self, line: &str);
    fn push_block(&mut self, block: Block);
}

impl Container for Function {
    fn line(&mut self, line: &str) {
        Function::line(self, line);
    }

    fn push_block(&mut self, block: Block) {
        Function::push_block(self, block);
    }
}

impl Container for Block {
    fn line(&mut self, line: &str) {
        Block::line(self, line);
    }

    fn push_block(&mut self, block: Block) {
        Block::push_block(self, block);
    }
}
