use crate::gen::RustCodeGenerator;
use crate::model::rust::{DataEnum, DataVariant};
use crate::model::rust::{Field, PlainEnum};
use crate::model::Model;
use crate::model::Range;
use crate::model::Rust;
use crate::model::RustType;
use crate::model::{Definition, Size};

const FOREIGN_KEY_DEFAULT_COLUMN: &str = "id";
const TUPLE_LIST_ENTRY_PARENT_COLUMN: &str = "list";
const TUPLE_LIST_ENTRY_VALUE_COLUMN: &str = "value";

#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[allow(clippy::module_name_repetitions)]
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
    BitsReprByByteArrayAndBitsLen,
    References(String, String, Option<Action>, Option<Action>),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Action {
    Cascade,
    Restrict,
}

impl ToString for Action {
    fn to_string(&self) -> String {
        match self {
            Action::Cascade => "CASCADE",
            Action::Restrict => "RESTRICT",
        }
        .into()
    }
}

impl SqlType {
    pub fn nullable(self) -> Self {
        match self {
            SqlType::NotNull(inner) => *inner,
            other => other,
        }
    }

    pub fn not_null(self) -> Self {
        SqlType::NotNull(Box::new(self))
    }

    pub fn to_rust(&self) -> RustType {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
        RustType::Option(Box::new(match self {
            SqlType::SmallInt => RustType::I16(Range::inclusive(0, i16::max_value())),
            SqlType::Integer => RustType::I32(Range::inclusive(0, i32::max_value())),
            SqlType::BigInt => RustType::I64(Range::inclusive(0, i64::max_value())),
            SqlType::Serial => RustType::I32(Range::inclusive(0, i32::max_value())),
            SqlType::Boolean => RustType::Bool,
            SqlType::Text => RustType::String,
            SqlType::Array(inner) => RustType::Vec(Box::new(inner.to_rust())),
            SqlType::NotNull(inner) => return inner.to_rust().no_option(),
            SqlType::ByteArray => RustType::VecU8(Size::Any),
            SqlType::BitsReprByByteArrayAndBitsLen => RustType::BitVec(Size::Any),
            SqlType::References(name, _, _, _) => RustType::Complex(name.clone()),
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
            SqlType::ByteArray | SqlType::BitsReprByByteArrayAndBitsLen => "BYTEA".into(),
            SqlType::References(table, column, on_delete, on_update) => format!(
                "INTEGER REFERENCES {}({}){}{}",
                table,
                column,
                if let Some(cascade) = on_delete {
                    format!(" ON DELETE {}", cascade.to_string())
                } else {
                    "".into()
                },
                if let Some(cascade) = on_update {
                    format!(" ON UPDATE {}", cascade.to_string())
                } else {
                    "".into()
                },
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub sql: SqlType,
    pub primary_key: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    CombinedPrimaryKey(Vec<String>),
    OneNotNull(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Sql {
    Table(Vec<Column>, Vec<Constraint>),
    Enum(Vec<String>),
    Index(String, Vec<String>),
    /// Table bein affected to ->
    AbandonChildrenFunction(String, Vec<(String, String, String)>),
    SilentlyPreventAnyDelete(String),
}

impl Model<Sql> {
    pub fn convert_rust_to_sql(rust_model: &Model<Rust>) -> Model<Sql> {
        let mut model = Model {
            name: rust_model.name.clone(),
            oid: rust_model.oid.clone(),
            imports: Default::default(), // ignored in SQL
            definitions: Vec::with_capacity(rust_model.definitions.len()),
        };
        for Definition(name, rust) in &rust_model.definitions {
            Self::definition_to_sql(name, rust, &mut model.definitions);
        }
        model
    }

    fn definition_to_sql(name: &str, rust: &Rust, definitions: &mut Vec<Definition<Sql>>) {
        match rust {
            Rust::Struct {
                fields,
                extension_after: _,
            } => Self::rust_struct_to_sql_table(name, fields, definitions),
            Rust::Enum(rust_enum) => Self::rust_enum_to_sql_enum(name, rust_enum, definitions),
            Rust::DataEnum(enumeration) => {
                Self::rust_data_enum_to_sql_table(name, enumeration, definitions)
            }
            Rust::TupleStruct { r#type: rust, .. } => {
                Self::rust_tuple_struct_to_sql_table(name, rust, definitions)
            }
        }
    }

    pub fn rust_struct_to_sql_table(
        name: &str,
        fields: &[Field],
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        let mut deferred = Vec::default();
        let mut columns = Vec::with_capacity(fields.len() + 1);
        columns.push(Column {
            name: FOREIGN_KEY_DEFAULT_COLUMN.into(),
            sql: SqlType::Serial,
            primary_key: true,
        });
        for field in fields {
            if field.r#type().is_vec() {
                let list_entry_name = Self::struct_list_entry_table_name(name, field.name());
                let value_sql_type = field.r#type().clone().into_inner_type().to_sql();
                Self::add_list_table(name, &mut deferred, &list_entry_name, &value_sql_type);
            } else {
                columns.push(Column {
                    name: Self::sql_column_name(field.name()),
                    sql: field.r#type().to_sql(),
                    primary_key: false,
                });
            }
        }
        definitions.push(Definition(
            name.into(),
            Sql::Table(columns, Default::default()),
        ));

        Self::append_index_and_abandon_function(
            name,
            fields.iter().map(Field::fallback_representation),
            definitions,
        );
        deferred.into_iter().for_each(|e| definitions.push(e));
    }

    pub fn rust_data_enum_to_sql_table(
        name: &str,
        enumeration: &DataEnum,
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        let mut columns = Vec::with_capacity(enumeration.len() + 1);
        // TODO
        if !enumeration
            .variants()
            .map(|variant| FOREIGN_KEY_DEFAULT_COLUMN.eq_ignore_ascii_case(variant.name()))
            .any(|found| found)
        {
            columns.push(Column {
                name: FOREIGN_KEY_DEFAULT_COLUMN.into(),
                sql: SqlType::Serial,
                primary_key: true,
            });
        }
        for variant in enumeration.variants() {
            columns.push(Column {
                name: Self::sql_column_name(variant.name()),
                sql: variant.r#type().to_sql().nullable(),
                primary_key: false,
            });
        }
        definitions.push(Definition(
            name.into(),
            Sql::Table(
                columns,
                vec![Constraint::OneNotNull(
                    enumeration
                        .variants()
                        .map(|variant| RustCodeGenerator::rust_module_name(variant.name()))
                        .collect::<Vec<String>>(),
                )],
            ),
        ));

        Self::append_index_and_abandon_function(
            name,
            enumeration
                .variants()
                .map(DataVariant::fallback_representation),
            definitions,
        );
    }

    fn add_index_if_applicable(
        table: &str,
        column: &str,
        rust: &RustType,
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        if let SqlType::References(..) = rust.to_sql().nullable() {
            definitions.push(Definition(
                String::default(),
                Sql::Index(table.into(), vec![column.into()]),
            ));
        }
    }

    pub fn rust_enum_to_sql_enum(
        name: &str,
        enumeration: &PlainEnum,
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        let variants = enumeration.variants().map(String::clone).collect();
        definitions.push(Definition(name.into(), Sql::Enum(variants)));
        Self::add_silently_prevent_any_delete(name, definitions);
    }

    pub fn rust_tuple_struct_to_sql_table(
        name: &str,
        rust_inner: &RustType,
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        {
            definitions.push(Definition(
                name.into(),
                Sql::Table(
                    vec![Column {
                        name: FOREIGN_KEY_DEFAULT_COLUMN.into(),
                        sql: SqlType::Serial,
                        primary_key: true,
                    }],
                    Default::default(),
                ),
            ));
        }
        {
            let list_entry_name = format!("{}ListEntry", name);
            let value_sql_type = rust_inner.clone().into_inner_type().to_sql();
            Self::add_list_table(name, definitions, &list_entry_name, &value_sql_type);
        }
    }

    fn add_list_table(
        name: &str,
        definitions: &mut Vec<Definition<Sql>>,
        list_entry_name: &str,
        value_sql_type: &SqlType,
    ) {
        definitions.push(Definition(
            list_entry_name.to_string(),
            Sql::Table(
                vec![
                    Column {
                        name: TUPLE_LIST_ENTRY_PARENT_COLUMN.into(),
                        sql: SqlType::NotNull(Box::new(SqlType::References(
                            name.into(),
                            FOREIGN_KEY_DEFAULT_COLUMN.into(),
                            Some(Action::Cascade),
                            Some(Action::Cascade),
                        ))),
                        primary_key: false,
                    },
                    Column {
                        name: TUPLE_LIST_ENTRY_VALUE_COLUMN.into(),
                        sql: value_sql_type.clone(),
                        primary_key: false,
                    },
                ],
                vec![Constraint::CombinedPrimaryKey(vec![
                    TUPLE_LIST_ENTRY_PARENT_COLUMN.into(),
                    TUPLE_LIST_ENTRY_VALUE_COLUMN.into(),
                ])],
            ),
        ));
        definitions.push(Definition(
            Default::default(),
            Sql::Index(
                list_entry_name.to_string(),
                vec![TUPLE_LIST_ENTRY_PARENT_COLUMN.into()],
            ),
        ));
        definitions.push(Definition(
            Default::default(),
            Sql::Index(
                list_entry_name.to_string(),
                vec![TUPLE_LIST_ENTRY_VALUE_COLUMN.into()],
            ),
        ));
        if let SqlType::References(other_table, other_column, ..) =
            value_sql_type.clone().nullable()
        {
            Self::add_abandon_children(
                list_entry_name,
                vec![(
                    TUPLE_LIST_ENTRY_VALUE_COLUMN.into(),
                    other_table,
                    other_column,
                )],
                definitions,
            );
        }
    }

    pub fn sql_column_name(name: &str) -> String {
        if FOREIGN_KEY_DEFAULT_COLUMN.eq_ignore_ascii_case(name.trim()) {
            let mut string = RustCodeGenerator::rust_module_name(name);
            string.push('_');
            string
        } else {
            RustCodeGenerator::rust_module_name(name)
        }
    }

    fn append_index_and_abandon_function<'a>(
        name: &str,
        fields: impl Iterator<Item = &'a (String, RustType)>,
        definitions: &mut Vec<Definition<Sql>>,
    ) {
        let mut children = Vec::new();
        for (column, rust) in fields {
            let column = Self::sql_column_name(column);
            Self::add_index_if_applicable(name, &column, rust, definitions);
            if let SqlType::References(other_table, other_column, _, _) = rust.to_sql().nullable() {
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
            format!("DelChilds_{}", name),
            Sql::AbandonChildrenFunction(name.into(), children),
        ));
    }

    fn add_silently_prevent_any_delete(name: &str, definitions: &mut Vec<Definition<Sql>>) {
        definitions.push(Definition(
            format!("SilentlyPreventAnyDeleteOn{}", name),
            Sql::SilentlyPreventAnyDelete(name.into()),
        ));
    }

    pub fn has_no_column_in_embedded_struct(rust: &RustType) -> bool {
        rust.is_vec()
    }

    pub fn is_primitive(rust: &RustType) -> bool {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
        match rust.clone().into_inner_type() {
            RustType::String => true,
            RustType::VecU8(_) => true,
            RustType::BitVec(_) => true,
            r => r.is_primitive(),
        }
    }

    pub fn struct_list_entry_table_name(struct_name: &str, field_name: &str) -> String {
        format!(
            "{}_{}",
            struct_name,
            RustCodeGenerator::rust_variant_name(field_name)
        )
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

#[allow(clippy::module_name_repetitions)]
pub trait ToSql {
    fn to_sql(&self) -> SqlType;
}

impl ToSql for RustType {
    fn to_sql(&self) -> SqlType {
        SqlType::NotNull(Box::new(match self {
            RustType::Bool => SqlType::Boolean,
            RustType::U8(_) | RustType::I8(_) => SqlType::SmallInt,
            RustType::U16(Range(_, upper, _)) if *upper <= i16::max_value() as u16 => {
                SqlType::SmallInt
            }
            RustType::I16(_) => SqlType::SmallInt,
            RustType::U32(Range(_, upper, _)) if *upper <= i32::max_value() as u32 => {
                SqlType::Integer
            }
            RustType::U16(_) | RustType::I32(_) => SqlType::Integer,
            RustType::U32(_) | RustType::U64(_) | RustType::I64(_) => SqlType::BigInt,
            RustType::String => SqlType::Text,
            RustType::VecU8(_) => SqlType::ByteArray,
            RustType::BitVec(_) => SqlType::BitsReprByByteArrayAndBitsLen,
            RustType::Vec(inner) => SqlType::Array(inner.to_sql().into()),
            RustType::Option(inner) => return inner.to_sql().nullable(),
            RustType::Complex(name) => SqlType::References(
                name.clone(),
                FOREIGN_KEY_DEFAULT_COLUMN.into(),
                Some(Action::Cascade),
                Some(Action::Cascade),
            ),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::rust::Field;
    use crate::model::Model;
    use crate::model::{Import, Size};

    #[test]
    fn test_conversion_struct() {
        let model = Model {
            name: "Manfred".into(),
            oid: None,
            imports: vec![Import {
                what: vec!["a".into(), "b".into()],
                from: "to_be_ignored".into(),
                from_oid: None,
            }],
            definitions: vec![Definition(
                "Person".into(),
                Rust::struct_from_fields(vec![
                    Field::from_name_type("name", RustType::String),
                    Field::from_name_type("birth", RustType::Complex("City".into())),
                ]),
            )],
        }
        .to_sql();
        assert_eq!("Manfred", &model.name);
        assert!(model.imports.is_empty());
        assert_eq!(
            &vec![
                Definition(
                    "Person".into(),
                    Sql::Table(
                        vec![
                            Column {
                                name: "id".into(),
                                sql: SqlType::Serial,
                                primary_key: true
                            },
                            Column {
                                name: "name".into(),
                                sql: SqlType::NotNull(SqlType::Text.into()),
                                primary_key: false
                            },
                            Column {
                                name: "birth".into(),
                                sql: SqlType::NotNull(
                                    SqlType::References(
                                        "City".into(),
                                        FOREIGN_KEY_DEFAULT_COLUMN.into(),
                                        Some(Action::Cascade),
                                        Some(Action::Cascade),
                                    )
                                    .into()
                                ),
                                primary_key: false
                            },
                        ],
                        vec![]
                    )
                ),
                Definition(
                    String::default(),
                    Sql::Index("Person".into(), vec!["birth".into()])
                ),
                Definition(
                    "DelChilds_Person".into(),
                    Sql::AbandonChildrenFunction(
                        "Person".into(),
                        vec![("birth".into(), "City".into(), "id".into())],
                    )
                )
            ],
            &model.definitions
        );
    }

    #[test]
    fn test_conversion_data_enum() {
        let model = Model {
            name: "Hurray".into(),
            oid: None,
            imports: vec![Import {
                what: vec!["a".into(), "b".into()],
                from: "to_be_ignored".into(),
                from_oid: None,
            }],
            definitions: vec![Definition(
                "PersonState".into(),
                Rust::DataEnum(
                    vec![
                        DataVariant::from_name_type("DeadSince", RustType::String),
                        DataVariant::from_name_type("Alive", RustType::Complex("Person".into())),
                    ]
                    .into(),
                ),
            )],
        }
        .to_sql();
        assert_eq!("Hurray", &model.name);
        assert!(model.imports.is_empty());
        assert_eq!(
            &vec![
                Definition(
                    "PersonState".into(),
                    Sql::Table(
                        vec![
                            Column {
                                name: "id".into(),
                                sql: SqlType::Serial,
                                primary_key: true
                            },
                            Column {
                                name: "dead_since".into(),
                                sql: SqlType::Text,
                                primary_key: false
                            },
                            Column {
                                name: "alive".into(),
                                sql: SqlType::References(
                                    "Person".into(),
                                    FOREIGN_KEY_DEFAULT_COLUMN.into(),
                                    Some(Action::Cascade),
                                    Some(Action::Cascade),
                                ),
                                primary_key: false
                            },
                        ],
                        vec![Constraint::OneNotNull(vec![
                            "dead_since".into(),
                            "alive".into(),
                        ])]
                    )
                ),
                Definition(
                    String::default(),
                    Sql::Index("PersonState".into(), vec!["alive".into()])
                ),
                Definition(
                    "DelChilds_PersonState".into(),
                    Sql::AbandonChildrenFunction(
                        "PersonState".into(),
                        vec![("alive".into(), "Person".into(), "id".into())],
                    )
                )
            ],
            &model.definitions
        );
    }

    #[test]
    fn test_conversion_enum() {
        let model = Model {
            name: "Alfred".into(),
            oid: None,
            imports: vec![Import {
                what: vec!["a".into(), "b".into()],
                from: "to_be_ignored".into(),
                from_oid: None,
            }],
            definitions: vec![Definition(
                "City".into(),
                Rust::Enum(vec!["Esslingen".into(), "Stuttgart".into()].into()),
            )],
        }
        .to_sql();
        assert_eq!("Alfred", &model.name);
        assert!(model.imports.is_empty());
        assert_eq!(
            &vec![
                Definition(
                    "City".into(),
                    Sql::Enum(vec!["Esslingen".into(), "Stuttgart".into(),],)
                ),
                Definition(
                    "SilentlyPreventAnyDeleteOnCity".into(),
                    Sql::SilentlyPreventAnyDelete("City".into())
                )
            ],
            &model.definitions
        );
    }

    #[test]
    fn test_conversion_struct_with_vec() {
        let model = Model {
            name: "Bernhard".into(),
            oid: None,
            imports: vec![],
            definitions: vec![Definition(
                "SomeStruct".into(),
                Rust::struct_from_fields(vec![
                    Field::from_name_type(
                        "list_of_primitive",
                        RustType::Vec(Box::new(RustType::String)),
                    ),
                    Field::from_name_type(
                        "list_of_reference",
                        RustType::Vec(Box::new(RustType::Complex("ComplexType".into()))),
                    ),
                ]),
            )],
        }
        .to_sql();
        assert_eq!("Bernhard", &model.name);
        assert!(model.imports.is_empty());
        assert_eq!(
            &model.definitions,
            &vec![
                Definition(
                    "SomeStruct".into(),
                    Sql::Table(
                        vec![Column {
                            name: "id".into(),
                            sql: SqlType::Serial,
                            primary_key: true
                        }],
                        vec![],
                    )
                ),
                Definition(
                    "SomeStruct_ListOfPrimitive".into(),
                    Sql::Table(
                        vec![
                            Column {
                                name: "list".into(),
                                sql: SqlType::References(
                                    "SomeStruct".into(),
                                    "id".into(),
                                    Some(Action::Cascade),
                                    Some(Action::Cascade),
                                )
                                .not_null(),
                                primary_key: false,
                            },
                            Column {
                                name: "value".into(),
                                sql: SqlType::Text.not_null(),
                                primary_key: false,
                            },
                        ],
                        vec![Constraint::CombinedPrimaryKey(vec![
                            "list".into(),
                            "value".into()
                        ])]
                    )
                ),
                Definition(
                    String::default(),
                    Sql::Index("SomeStruct_ListOfPrimitive".into(), vec!["list".into()])
                ),
                Definition(
                    String::default(),
                    Sql::Index("SomeStruct_ListOfPrimitive".into(), vec!["value".into()])
                ),
                Definition(
                    "SomeStruct_ListOfReference".into(),
                    Sql::Table(
                        vec![
                            Column {
                                name: "list".into(),
                                sql: SqlType::References(
                                    "SomeStruct".into(),
                                    "id".into(),
                                    Some(Action::Cascade),
                                    Some(Action::Cascade),
                                )
                                .not_null(),
                                primary_key: false,
                            },
                            Column {
                                name: "value".into(),
                                sql: SqlType::References(
                                    "ComplexType".into(),
                                    "id".into(),
                                    Some(Action::Cascade),
                                    Some(Action::Cascade)
                                )
                                .not_null(),
                                primary_key: false,
                            },
                        ],
                        vec![Constraint::CombinedPrimaryKey(vec![
                            "list".into(),
                            "value".into()
                        ])]
                    )
                ),
                Definition(
                    String::default(),
                    Sql::Index("SomeStruct_ListOfReference".into(), vec!["list".into()])
                ),
                Definition(
                    String::default(),
                    Sql::Index("SomeStruct_ListOfReference".into(), vec!["value".into()])
                ),
                Definition(
                    "DelChilds_SomeStruct_ListOfReference".into(),
                    Sql::AbandonChildrenFunction(
                        "SomeStruct_ListOfReference".into(),
                        vec![("value".into(), "ComplexType".into(), "id".into())]
                    )
                )
            ],
        );
    }

    #[test]
    fn test_conversion_tuple_struct() {
        let model = Model {
            name: "Hurray".into(),
            oid: None,
            imports: vec![Import {
                what: vec!["a".into(), "b".into()],
                from: "to_be_ignored".into(),
                from_oid: None,
            }],
            definitions: vec![
                Definition(
                    "Whatever".into(),
                    Rust::tuple_struct_from_type(RustType::String),
                ),
                Definition(
                    "Whatelse".into(),
                    Rust::tuple_struct_from_type(RustType::Complex("Whatever".into())),
                ),
            ],
        }
        .to_sql();
        assert_eq!("Hurray", &model.name);
        assert!(model.imports.is_empty());
        assert_eq!(
            &vec![
                Definition(
                    "Whatever".into(),
                    Sql::Table(
                        vec![Column {
                            name: "id".into(),
                            sql: SqlType::Serial,
                            primary_key: true
                        },],
                        vec![]
                    )
                ),
                Definition(
                    "WhateverListEntry".into(),
                    Sql::Table(
                        vec![
                            Column {
                                name: "list".into(),
                                sql: SqlType::NotNull(
                                    SqlType::References(
                                        "Whatever".into(),
                                        "id".into(),
                                        Some(Action::Cascade),
                                        Some(Action::Cascade),
                                    )
                                    .into()
                                ),
                                primary_key: false
                            },
                            Column {
                                name: "value".into(),
                                sql: SqlType::NotNull(SqlType::Text.into()),
                                primary_key: false
                            },
                        ],
                        vec![Constraint::CombinedPrimaryKey(vec![
                            "list".into(),
                            "value".into()
                        ])]
                    )
                ),
                Definition(
                    String::new(),
                    Sql::Index(
                        "WhateverListEntry".into(),
                        vec![TUPLE_LIST_ENTRY_PARENT_COLUMN.into()]
                    )
                ),
                Definition(
                    String::new(),
                    Sql::Index(
                        "WhateverListEntry".into(),
                        vec![TUPLE_LIST_ENTRY_VALUE_COLUMN.into()]
                    )
                ),
                Definition(
                    "Whatelse".into(),
                    Sql::Table(
                        vec![Column {
                            name: "id".into(),
                            sql: SqlType::Serial,
                            primary_key: true
                        },],
                        vec![]
                    )
                ),
                Definition(
                    "WhatelseListEntry".into(),
                    Sql::Table(
                        vec![
                            Column {
                                name: TUPLE_LIST_ENTRY_PARENT_COLUMN.into(),
                                sql: SqlType::NotNull(
                                    SqlType::References(
                                        "Whatelse".into(),
                                        "id".into(),
                                        Some(Action::Cascade),
                                        Some(Action::Cascade),
                                    )
                                    .into()
                                ),
                                primary_key: false
                            },
                            Column {
                                name: TUPLE_LIST_ENTRY_VALUE_COLUMN.into(),
                                sql: SqlType::NotNull(
                                    SqlType::References(
                                        "Whatever".into(),
                                        FOREIGN_KEY_DEFAULT_COLUMN.into(),
                                        Some(Action::Cascade),
                                        Some(Action::Cascade),
                                    )
                                    .into()
                                ),
                                primary_key: false
                            },
                        ],
                        vec![Constraint::CombinedPrimaryKey(vec![
                            TUPLE_LIST_ENTRY_PARENT_COLUMN.into(),
                            TUPLE_LIST_ENTRY_VALUE_COLUMN.into()
                        ])]
                    )
                ),
                Definition(
                    String::new(),
                    Sql::Index(
                        "WhatelseListEntry".into(),
                        vec![TUPLE_LIST_ENTRY_PARENT_COLUMN.into()]
                    )
                ),
                Definition(
                    String::new(),
                    Sql::Index(
                        "WhatelseListEntry".into(),
                        vec![TUPLE_LIST_ENTRY_VALUE_COLUMN.into()]
                    )
                ),
                Definition(
                    "DelChilds_WhatelseListEntry".into(),
                    Sql::AbandonChildrenFunction(
                        "WhatelseListEntry".into(),
                        vec![(
                            TUPLE_LIST_ENTRY_VALUE_COLUMN.into(),
                            "Whatever".into(),
                            "id".into()
                        )],
                    )
                )
            ],
            &model.definitions
        );
    }

    #[test]
    fn test_conversion_on_first_level_name_clash() {
        let model = Model {
            name: "Alfred".into(),
            oid: None,
            imports: vec![Import {
                what: vec!["a".into(), "b".into()],
                from: "to_be_ignored".into(),
                from_oid: None,
            }],
            definitions: vec![Definition(
                "City".into(),
                Rust::struct_from_fields(vec![Field::from_name_type("id", RustType::String)]),
            )],
        }
        .to_sql();
        assert_eq!("Alfred", &model.name);
        assert!(model.imports.is_empty());
        assert_eq!(
            &vec![Definition(
                "City".into(),
                Sql::Table(
                    vec![
                        Column {
                            name: FOREIGN_KEY_DEFAULT_COLUMN.into(),
                            sql: SqlType::Serial,
                            primary_key: true
                        },
                        Column {
                            name: "id_".into(),
                            sql: SqlType::NotNull(SqlType::Text.into()),
                            primary_key: false
                        },
                    ],
                    vec![]
                )
            ),],
            &model.definitions
        );
    }

    #[test]
    fn test_rust_to_sql_to_rust() {
        assert_eq!(RustType::Bool.to_sql().to_rust(), RustType::Bool);
        assert_eq!(
            RustType::I8(Range::inclusive(0, i8::max_value()))
                .to_sql()
                .to_rust(),
            RustType::I16(Range::inclusive(0, i16::max_value()))
        );
        assert_eq!(
            RustType::U8(Range::inclusive(0, u8::max_value()))
                .to_sql()
                .to_rust(),
            RustType::I16(Range::inclusive(0, i16::max_value()))
        );
        assert_eq!(
            RustType::I16(Range::inclusive(0, i16::max_value()))
                .to_sql()
                .to_rust(),
            RustType::I16(Range::inclusive(0, i16::max_value()))
        );
        assert_eq!(
            RustType::U16(Range::inclusive(0, i16::max_value() as u16))
                .to_sql()
                .to_rust(),
            RustType::I16(Range::inclusive(0, i16::max_value()))
        );
        assert_eq!(
            RustType::U16(Range::inclusive(0, u16::max_value()))
                .to_sql()
                .to_rust(),
            RustType::I32(Range::inclusive(0, i32::max_value()))
        );
        assert_eq!(
            RustType::I32(Range::inclusive(0, i32::max_value()))
                .to_sql()
                .to_rust(),
            RustType::I32(Range::inclusive(0, i32::max_value()))
        );
        assert_eq!(
            RustType::U32(Range::inclusive(0, i32::max_value() as u32))
                .to_sql()
                .to_rust(),
            RustType::I32(Range::inclusive(0, i32::max_value()))
        );
        assert_eq!(
            RustType::U32(Range::inclusive(0, u32::max_value()))
                .to_sql()
                .to_rust(),
            RustType::I64(Range::inclusive(0, i64::max_value()))
        );
        assert_eq!(
            RustType::I64(Range::inclusive(0, i64::max_value()))
                .to_sql()
                .to_rust(),
            RustType::I64(Range::inclusive(0, i64::max_value()))
        );
        assert_eq!(
            RustType::U64(Range::none()).to_sql().to_rust(),
            RustType::I64(Range::inclusive(0, i64::max_value()))
        );
        assert_eq!(
            RustType::U64(Range::inclusive(Some(0), Some(u64::max_value())))
                .to_sql()
                .to_rust(),
            RustType::I64(Range::inclusive(0, i64::max_value()))
        );

        assert_eq!(RustType::String.to_sql().to_rust(), RustType::String,);
        assert_eq!(
            RustType::VecU8(Size::Any).to_sql().to_rust(),
            RustType::VecU8(Size::Any)
        );
        assert_eq!(
            RustType::Vec(Box::new(RustType::String)).to_sql().to_rust(),
            RustType::Vec(Box::new(RustType::String)),
        );
        assert_eq!(
            RustType::Option(Box::new(RustType::VecU8(Size::Any)))
                .to_sql()
                .to_rust(),
            RustType::Option(Box::new(RustType::VecU8(Size::Any))),
        );
        assert_eq!(
            RustType::Complex("MuchComplex".into()).to_sql().to_rust(),
            RustType::Complex("MuchComplex".into()),
        );
    }

    #[test]
    fn test_sql_to_rust() {
        // only cases that are not already tested by above
        assert_eq!(
            SqlType::NotNull(SqlType::Serial.into()).to_rust(),
            RustType::I32(Range::inclusive(0, i32::max_value()))
        );
    }

    #[test]
    fn test_nullable() {
        assert_eq!(
            SqlType::NotNull(SqlType::Serial.into()).nullable(),
            SqlType::Serial
        );
        assert_eq!(SqlType::Serial.nullable(), SqlType::Serial);
    }

    #[test]
    fn test_to_string() {
        assert_eq!("SMALLINT", &SqlType::SmallInt.to_string());
        assert_eq!("INTEGER", &SqlType::Integer.to_string());
        assert_eq!("BIGINT", &SqlType::BigInt.to_string());
        assert_eq!("SERIAL", &SqlType::Serial.to_string());
        assert_eq!("BOOLEAN", &SqlType::Boolean.to_string());
        assert_eq!("TEXT", &SqlType::Text.to_string());
        assert_eq!(
            "SMALLINT[]",
            &SqlType::Array(SqlType::SmallInt.into()).to_string()
        );
        assert_eq!(
            "TEXT NOT NULL",
            &SqlType::NotNull(SqlType::Text.into()).to_string()
        );
        assert_eq!(
            "INTEGER REFERENCES tablo(columno)",
            &SqlType::References("tablo".into(), "columno".into(), None, None).to_string()
        );
        assert_eq!(
            "INTEGER REFERENCES tablo(columno) ON DELETE CASCADE ON UPDATE RESTRICT",
            &SqlType::References(
                "tablo".into(),
                "columno".into(),
                Some(Action::Cascade),
                Some(Action::Restrict),
            )
            .to_string()
        );
        assert_eq!(
            "INTEGER REFERENCES table(column) NOT NULL",
            &SqlType::NotNull(
                SqlType::References("table".into(), "column".into(), None, None).into()
            )
            .to_string()
        );
        assert_eq!(
            "INTEGER REFERENCES table(column) ON DELETE RESTRICT ON UPDATE CASCADE NOT NULL",
            &SqlType::NotNull(
                SqlType::References(
                    "table".into(),
                    "column".into(),
                    Some(Action::Restrict),
                    Some(Action::Cascade),
                )
                .into()
            )
            .to_string()
        );
    }
}
