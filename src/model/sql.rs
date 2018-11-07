use model::Definition;
use model::Model;
use model::Range;
use model::Rust;
use model::RustType;

const FOREIGN_KEY_DEFAULT_COLUMN: &str = "id";
const TUPLE_LIST_ENTRY_PARENT_COLUMN: &str = "list";
const TUPLE_LIST_ENTRY_VALUE_COLUMN: &str = "value";

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SqlType {
    SmallInt, // 2byte
    Integer,  // 4byte
    BigInt,   // 8byte
    Serial,   // 4byte
    Boolean,
    Text,
    Array(Box<SqlType>),
    NotNull(Box<SqlType>),
    ByteArray,
    References(String, String),
}

impl SqlType {
    pub fn nullable(self) -> Self {
        match self {
            SqlType::NotNull(inner) => *inner,
            other => other,
        }
    }

    pub fn to_rust(&self) -> RustType {
        RustType::Option(Box::new(match self {
            SqlType::SmallInt => RustType::I16(Range(0, ::std::i16::MAX)),
            SqlType::Integer => RustType::I32(Range(0, ::std::i32::MAX)),
            SqlType::BigInt => RustType::I64(Range(0, ::std::i64::MAX)),
            SqlType::Serial => RustType::I32(Range(0, ::std::i32::MAX)),
            SqlType::Boolean => RustType::Bool,
            SqlType::Text => RustType::String,
            SqlType::Array(inner) => RustType::Vec(Box::new(inner.to_rust())),
            SqlType::NotNull(inner) => return inner.to_rust().no_option(),
            SqlType::ByteArray => RustType::VecU8,
            SqlType::References(name, _) => RustType::Complex(name.clone()),
        }))
    }
}

impl ToString for SqlType {
    fn to_string(&self) -> String {
        match self {
            SqlType::SmallInt => "SMALLINT".into(),
            SqlType::Integer => "INTEGER".into(),
            SqlType::BigInt => "BIGINT".into(),
            SqlType::Serial => "SERIAL".into(),
            SqlType::Boolean => "BOOLEAN".into(),
            SqlType::Text => "TEXT".into(),
            SqlType::Array(inner) => format!("{}[]", inner.to_string()),
            SqlType::NotNull(inner) => format!("{} NOT NULL", inner.to_string()),
            SqlType::ByteArray => "BYTEA".into(),
            SqlType::References(table, column) => {
                format!("INTEGER REFERENCES {}({}) ON DELETE CASCADE ON UPDATE CASCADE", table, column)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub sql: SqlType,
    pub primary_key: bool,
}

#[derive(Debug, Clone)]
pub enum Constraint {
    CombinedPrimaryKey(Vec<String>),
    OneNotNull(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum Sql {
    Table((Vec<Column>, Vec<Constraint>)),
    Enum(Vec<String>),
    Index(String, Vec<String>),
    AbandonChildrenFunction(String, Vec<(String, String, String)>),
}

impl Model<Sql> {
    pub fn convert_rust_to_sql(rust_model: &Model<Rust>) -> Model<Sql> {
        let mut model = Model {
            name: rust_model.name.clone(),
            imports: Default::default(), // ignored in SQL
            definitions: Vec::with_capacity(rust_model.definitions.len()),
        };
        for Definition(name, rust) in &rust_model.definitions {
            Self::definition_to_sql(&name, rust, &mut model.definitions);
        }
        model
    }

    fn definition_to_sql(name: &str, rust: &Rust, definitions: &mut Vec<Definition<Sql>>) {
        match rust {
            Rust::Struct(fields) => Self::rust_struct_to_sql_table(name, fields, definitions),
            Rust::Enum(variants) => Self::rust_enum_to_sql_enum(name, variants, definitions),
            Rust::DataEnum(fields) => Self::rust_data_enum_to_sql_table(name, fields, definitions),
            Rust::TupleStruct(rust) => {
                Self::rust_tuple_struct_to_sql_table(name, rust, definitions)
            }
        }
    }

    pub fn rust_struct_to_sql_table(
        name: &str,
        fields: &[(String, RustType)],
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        let mut columns = Vec::with_capacity(fields.len() + 1);
        columns.push(Column {
            name: FOREIGN_KEY_DEFAULT_COLUMN.into(),
            sql: SqlType::Serial,
            primary_key: true,
        });
        for (column, rust) in fields {
            columns.push(Column {
                name: Self::sql_column_name(&column),
                sql: rust.to_sql(),
                primary_key: false,
            });
        }
        definitions.push(Definition(
            name.into(),
            Sql::Table((columns, Default::default())),
        ));

        Self::append_index_and_abandon_function(name, fields, definitions);
    }

    pub fn rust_data_enum_to_sql_table(
        name: &str,
        fields: &[(String, RustType)],
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        let mut columns = Vec::with_capacity(fields.len() + 1);
        // TODO
        if !fields
            .iter()
            .map(|(name, _)| FOREIGN_KEY_DEFAULT_COLUMN.eq_ignore_ascii_case(&name))
            .any(|found| found)
        {
            columns.push(Column {
                name: FOREIGN_KEY_DEFAULT_COLUMN.into(),
                sql: SqlType::Serial,
                primary_key: true,
            });
        }
        for (column, rust) in fields {
            columns.push(Column {
                name: Self::sql_column_name(&column),
                sql: rust.to_sql().nullable(),
                primary_key: false,
            });
        }
        definitions.push(Definition(
            name.into(),
            Sql::Table((
                columns,
                vec![Constraint::OneNotNull(
                    fields
                        .iter()
                        .map(|(name, _)| ::gen::RustCodeGenerator::rust_module_name(&name))
                        .collect::<Vec<String>>(),
                )],
            )),
        ));

        Self::append_index_and_abandon_function(name, fields, definitions);
    }

    fn add_index_if_applicable(
        table: &str,
        column: &str,
        rust: &RustType,
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        if let SqlType::References(_, _) = rust.to_sql().nullable() {
            definitions.push(Definition(
                String::default(),
                Sql::Index(table.into(), vec![column.into()]),
            ));
        }
    }

    pub fn rust_enum_to_sql_enum(
        name: &str,
        variants: &[String],
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        let variants = Vec::from(variants);
        definitions.push(Definition(name.into(), Sql::Enum(variants)));
    }

    pub fn rust_tuple_struct_to_sql_table(
        name: &str,
        rust_inner: &RustType,
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        {
            definitions.push(Definition(
                name.into(),
                Sql::Table((
                    vec![Column {
                        name: FOREIGN_KEY_DEFAULT_COLUMN.into(),
                        sql: SqlType::Serial,
                        primary_key: true,
                    }],
                    Default::default(),
                )),
            ));
        }
        {
            definitions.push(Definition(
                format!("{}ListEntry", name),
                Sql::Table((
                    vec![
                        Column {
                            name: TUPLE_LIST_ENTRY_PARENT_COLUMN.into(),
                            sql: SqlType::NotNull(Box::new(SqlType::References(
                                name.into(),
                                FOREIGN_KEY_DEFAULT_COLUMN.into(),
                            ))),
                            primary_key: false,
                        },
                        Column {
                            name: TUPLE_LIST_ENTRY_VALUE_COLUMN.into(),
                            sql: rust_inner.clone().into_inner_type().to_sql(),
                            primary_key: false,
                        },
                    ],
                    vec![Constraint::CombinedPrimaryKey(vec![
                        TUPLE_LIST_ENTRY_PARENT_COLUMN.into(),
                        TUPLE_LIST_ENTRY_VALUE_COLUMN.into(),
                    ])],
                )),
            ));
        }
    }

    pub fn sql_column_name(name: &str) -> String {
        if FOREIGN_KEY_DEFAULT_COLUMN.eq_ignore_ascii_case(name.trim()) {
            let mut string = ::gen::RustCodeGenerator::rust_module_name(name);
            string.push('_');
            string
        } else {
            ::gen::RustCodeGenerator::rust_module_name(name)
        }
    }

    fn append_index_and_abandon_function(
        name: &str,
        fields: &[(String, RustType)],
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        let mut children = Vec::new();
        for (column, rust) in fields {
            let column = Self::sql_column_name(column);
            Self::add_index_if_applicable(name, &column, rust, definitions);
            if let SqlType::References(other_table, other_column) = rust.to_sql().nullable() {
                children.push((column, other_table, other_column));
            }
        }
        if !children.is_empty() {
            Self::add_abandon_children(name, children, definitions);
        }
    }

    fn add_abandon_children(
        name: &str,
        children: Vec<(String, String, String)>,
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        definitions.push(Definition(
            format!("AbandonChildrenOf{}", name),
            Sql::AbandonChildrenFunction(name.into(), children),
        ));
    }
}

pub trait ToSqlModel {
    fn to_sql(&self) -> Model<Sql>;
}

impl ToSqlModel for Model<Rust> {
    fn to_sql(&self) -> Model<Sql> {
        Model::convert_rust_to_sql(self)
    }
}

pub trait ToSql {
    fn to_sql(&self) -> SqlType;
}

impl ToSql for RustType {
    fn to_sql(&self) -> SqlType {
        fn to_sql_internal(rust: &RustType) -> SqlType {
            match rust {
                RustType::Bool => SqlType::Boolean,
                RustType::U8(_) => SqlType::SmallInt,
                RustType::I8(_) => SqlType::SmallInt,
                RustType::U16(Range(_, upper)) if *upper < ::std::i16::MAX as u16 => {
                    SqlType::SmallInt
                }
                RustType::U16(_) => SqlType::Integer,
                RustType::I16(_) => SqlType::SmallInt,
                RustType::U32(Range(_, upper)) if *upper < ::std::i32::MAX as u32 => {
                    SqlType::Integer
                }
                RustType::U32(_) => SqlType::BigInt,
                RustType::I32(_) => SqlType::Integer,
                RustType::U64(_) => SqlType::BigInt,
                RustType::I64(_) => SqlType::BigInt,
                RustType::String => SqlType::Text,
                RustType::VecU8 => SqlType::ByteArray,
                RustType::Vec(inner) => SqlType::Array(to_sql_internal(inner).into()),
                RustType::Option(inner) => to_sql_internal(inner),
                RustType::Complex(name) => {
                    SqlType::References(name.clone(), FOREIGN_KEY_DEFAULT_COLUMN.into())
                }
            }
        }
        if let RustType::Option(_) = self {
            to_sql_internal(self)
        } else {
            SqlType::NotNull(Box::new(to_sql_internal(self)))
        }
    }
}
