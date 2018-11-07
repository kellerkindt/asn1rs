use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;
use gen::rust::GeneratorSupplement;
use gen::rust::RustCodeGenerator;
use model::sql::SqlType;
use model::sql::ToSql;
use model::Definition;
use model::Field;
use model::Rust;
use model::RustType;

const TRAIT_PSQL_INSERTABLE: &str = "PsqlInsertable";

pub struct PsqlInserter;
impl GeneratorSupplement<Rust> for PsqlInserter {
    fn add_imports(&self, scope: &mut Scope) {
        scope.import("asn1c::io::psql", "Error as PsqlError");
        scope.import("asn1c::io::psql", TRAIT_PSQL_INSERTABLE);
        scope.import("asn1c::io::psql", "Transaction");
    }

    fn impl_supplement(&self, scope: &mut Scope, Definition(name, rust): &Definition<Rust>) {
        let implementation = Self::new_impl(scope, &name);
        match rust {
            Rust::Struct(fields) => {
                Self::impl_struct_insert_statement(
                    Self::new_insert_statement_fn(implementation),
                    name,
                    &fields[..],
                );
                Self::impl_struct_insert_fn(Self::new_insert_fn(implementation), name, &fields[..]);
            }
            Rust::DataEnum(fields) => {
                Self::impl_struct_insert_statement(
                    Self::new_insert_statement_fn(implementation),
                    name,
                    &fields[..],
                );
                Self::impl_data_enum_insert_fn(
                    Self::new_insert_fn(implementation),
                    name,
                    &fields[..],
                );
            }
            Rust::Enum(_) => {
                Self::impl_enum_insert_statement(Self::new_insert_statement_fn(implementation));
                Self::impl_enum_insert_fn(Self::new_insert_fn(implementation));
            }
            Rust::TupleStruct(rust) => {
                Self::impl_tuple_insert_statement(
                    Self::new_insert_statement_fn(implementation),
                    name,
                );
                Self::impl_tuple_insert_fn(Self::new_insert_fn(implementation), name, rust);
            }
        }
    }
}

impl PsqlInserter {
    fn new_impl<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Impl {
        scope.new_impl(name).impl_trait(TRAIT_PSQL_INSERTABLE)
    }

    fn new_insert_statement_fn(implementation: &mut Impl) -> &mut Function {
        implementation
            .new_fn("insert_statement")
            .ret("&'static str")
    }

    fn new_insert_fn(implementation: &mut Impl) -> &mut Function {
        implementation
            .new_fn("insert_with")
            .arg_ref_self()
            .arg("transaction", "&Transaction")
            .ret("Result<i32, PsqlError>")
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
                    .map(|(name, _)| name.clone())
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

    fn impl_struct_insert_fn(function: &mut Function, name: &str, fields: &[(String, RustType)]) {
        let mut variables = Vec::with_capacity(fields.len());
        for (name, rust) in fields {
            let name = RustCodeGenerator::rust_field_name(name, true);
            let sql_primitive = match rust.clone().into_inner_type() {
                RustType::String => true,
                RustType::VecU8 => true,
                r => r.is_primitive(),
            };
            variables.push(format!("&{}", name));
            if sql_primitive {
                function.line(&format!(
                    "let {} = {}self.{};",
                    name,
                    if !rust.is_primitive() { "&" } else { "" },
                    name,
                ));
                let inner_sql = rust.clone().into_inner_type().to_sql();
                let inner_rust = rust.clone().into_inner_type();
                if inner_sql.to_rust().into_inner_type() != inner_rust {
                    function.line(format!(
                        "let {} = {}{} as {}{};",
                        name,
                        name,
                        if let RustType::Option(_) = rust {
                            ".map(|v| v"
                        } else {
                            ""
                        },
                        inner_sql.to_rust().into_inner_type().to_string(),
                        if let RustType::Option(_) = rust {
                            ")"
                        } else {
                            ""
                        },
                    ));
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
        function.line("let statement = transaction.prepare_cached(Self::insert_statement())?;");
        function.line(format!(
            "let result = statement.query(&[{}])?;",
            variables.join(", ")
        ));
        function.line("PsqlError::expect_returned_index(result)");
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
            let sql_primitive = Self::is_sql_primitive(rust.clone().into_inner_type());
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
        function.line("let statement = transaction.prepare_cached(Self::insert_statement())?;");
        function.line(format!(
            "let result = statement.query(&[{}])?;",
            variables.join(", ")
        ));
        function.line("PsqlError::expect_returned_index(result)");
    }

    fn impl_enum_insert_fn(function: &mut Function) {
        function.line("Ok(self.value_index() as i32)");
    }

    fn impl_tuple_insert_fn(function: &mut Function, name: &str, rust: &RustType) {
        function.line("let statement = transaction.prepare_cached(Self::insert_statement())?;");
        function.line("let result = statement.query(&[])?;");
        function.line("let list = PsqlError::expect_returned_index(result)?;");
        function.line(format!(
            "let statement = transaction.prepare_cached(\"{}\")?;",
            Self::list_entry_insert_statement(name)
        ));
        let mut block_for = Block::new("for value in &self.0");
        let sql_primitive = Self::is_sql_primitive(rust.clone().into_inner_type());
        if !sql_primitive {
            block_for.line("let value = value.insert_with(transaction)?;");
        } else {
            let inner_sql = rust.clone().into_inner_type().to_sql();
            let inner_rust = rust.clone().into_inner_type();
            if inner_sql.to_rust().into_inner_type() != inner_rust {
                block_for.line(format!(
                    "let value = *value{} as {}{};",
                    if let RustType::Option(_) = rust {
                        ".map(|v| v"
                    } else {
                        ""
                    },
                    inner_sql.to_rust().into_inner_type().to_string(),
                    if let RustType::Option(_) = rust {
                        ")"
                    } else {
                        ""
                    },
                ));
            }
        }
        block_for.line("statement.execute(&[&list, &value])?;");
        function.push_block(block_for);
        function.line("Ok(list)");
    }

    fn is_sql_primitive(rust: RustType) -> bool {
        match rust {
            RustType::String => true,
            r => r.is_primitive(),
        }
    }

    fn list_entry_insert_statement(name: &str) -> String {
        format!("INSERT INTO {}ListEntry(list, value) ($1, $2)", name)
    }
}
