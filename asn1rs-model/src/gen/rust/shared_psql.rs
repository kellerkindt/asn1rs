use crate::gen::RustCodeGenerator;
use crate::model::rust::DataEnum;
use crate::model::sql::Sql;
use crate::model::{Model, RustType};

pub(crate) fn select_statement_single(name: &str) -> String {
    format!("SELECT * FROM {} WHERE id = $1", name)
}

#[cfg(feature = "async-psql")]
pub(crate) fn select_statement_many(name: &str) -> String {
    format!("SELECT * FROM {} WHERE id = ANY($1)", name)
}

pub(crate) fn tuple_struct_insert_statement(name: &str) -> String {
    format!("INSERT INTO {} DEFAULT VALUES RETURNING id", name)
}

pub(crate) fn struct_insert_statement(name: &str, fields: &[(String, RustType)]) -> String {
    format!(
        "INSERT INTO {}({}) VALUES({}) RETURNING id",
        name,
        fields
            .iter()
            .filter_map(|(name, field)| if field.is_vec() {
                None
            } else {
                Some(Model::sql_column_name(name))
            })
            .collect::<Vec<String>>()
            .join(", "),
        fields
            .iter()
            .filter_map(|(name, field)| if field.is_vec() { None } else { Some(name) })
            .enumerate()
            .map(|(num, _)| format!("${}", num + 1))
            .collect::<Vec<String>>()
            .join(", "),
    )
}

pub(crate) fn data_enum_insert_statement(name: &str, enumeration: &DataEnum) -> String {
    format!(
        "INSERT INTO {}({}) VALUES({}) RETURNING id",
        name,
        enumeration
            .variants()
            .map(|variant| RustCodeGenerator::rust_module_name(variant.name()))
            .collect::<Vec<String>>()
            .join(", "),
        enumeration
            .variants()
            .enumerate()
            .map(|(num, _)| format!("${}", num + 1))
            .collect::<Vec<String>>()
            .join(", "),
    )
}

pub(crate) fn struct_list_entry_insert_statement(struct_name: &str, field_name: &str) -> String {
    format!(
        "INSERT INTO {}(list, value) VALUES ($1, $2)",
        Model::<Sql>::struct_list_entry_table_name(struct_name, field_name),
    )
}

pub(crate) fn struct_list_entry_select_referenced_value_statement(
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

pub(crate) fn struct_list_entry_select_value_statement(
    struct_name: &str,
    field_name: &str,
) -> String {
    let listentry_table = Model::<Sql>::struct_list_entry_table_name(struct_name, field_name);
    format!("SELECT value FROM {} WHERE list = $1", listentry_table,)
}

pub(crate) fn list_entry_insert_statement(name: &str) -> String {
    format!("INSERT INTO {}ListEntry(list, value) VALUES ($1, $2)", name)
}

pub(crate) fn list_entry_query_statement(name: &str, inner: &RustType) -> String {
    if Model::<Sql>::is_primitive(inner) {
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
