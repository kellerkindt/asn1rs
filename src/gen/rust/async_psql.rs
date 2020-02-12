use crate::gen::rust::GeneratorSupplement;
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
            .map(|(name, r_type)| insert_field(fun, name, r_type))
        {
            match insert {
                FieldInsert::AsyncVec(name) => {
                    to_await.push(name.clone());
                    params.push(name);
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
            let index = to_await.len();
            to_await.push("statement".to_string());
            let elements = to_await.join(", ");
            let elements_result = to_await.join("?, ");

            fun.line(&format!(
                "let ({}) = {}::try_join!({})?;",
                elements, MODULE_NAME, elements
            ));
        }

        for (primitive, conversion) in primitives {
            fun.line(&format!(
                "let {} = self.{}{}{};",
                primitive,
                primitive,
                conversion.as_ref().map(|_| " as ").unwrap_or_default(),
                conversion
                    .as_ref()
                    .map(|r| r.to_string())
                    .unwrap_or_default(),
            ));
        }

        // Ok(context.transaction().query_one(statement, &[]).await?.get(1))
        fun.line(format!(
            "Ok(context.transaction().query_one(&statement, &[{}]).await?.get(1))",
            params
                .iter()
                .map(|p| format!("&{}", p))
                .collect::<Vec<_>>()
                .join(", ")
        ));
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
            if Model::<Sql>::is_vec(field) {
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

fn insert_field(function: &mut Function, name: &str, r_type: &RustType) -> FieldInsert {
    if Model::<Sql>::is_vec(r_type) {
        function.line(&format!(
            "let {} = {}::try_join_all(self.{}.iter().map(|v| v.{}(context)));",
            name,
            MODULE_NAME,
            name,
            insert_fn_name()
        ));
        FieldInsert::AsyncVec(name.to_string())
    } else if r_type.is_primitive() {
        let rerust = r_type.to_sql().to_rust();
        FieldInsert::Primitive(
            name.to_string(),
            if rerust.ne(r_type) {
                Some(rerust)
            } else {
                None
            },
        )
    } else {
        function.line(&format!(
            "let {} = self.{}.{}(&context);",
            name,
            name,
            insert_fn_name()
        ));
        FieldInsert::AsyncComplex(name.to_string())
    }
}

enum FieldInsert {
    AsyncVec(String),
    AsyncComplex(String),
    Primitive(String, Option<RustType>),
}

impl FieldInsert {
    pub fn is_async(&self) -> bool {
        match self {
            FieldInsert::AsyncVec(_) => true,
            FieldInsert::AsyncComplex(_) => true,
            FieldInsert::Primitive(_, _) => false,
        }
    }

    pub fn is_vec(&self) -> bool {
        if let FieldInsert::AsyncVec(_) = self {
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

    pub fn name(&self) -> &str {
        match self {
            FieldInsert::AsyncVec(name) => name,
            FieldInsert::AsyncComplex(name) => name,
            FieldInsert::Primitive(name, _) => name,
        }
    }
}
