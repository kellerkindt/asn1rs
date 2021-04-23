use bytes::BytesMut;
use std::error::Error;

#[cfg(feature = "psql")]
use postgres::types::{FromSql, IsNull, ToSql, Type};

use crate::syn::null::Null;
#[cfg(all(feature = "async-psql", not(feature = "psql")))]
use tokio_postgres::types::{FromSql, IsNull, ToSql, Type};

impl<'a> FromSql<'a> for Null {
    fn from_sql(_ty: &Type, _raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(Null)
    }

    fn accepts(ty: &Type) -> bool {
        <Option<Vec<u8>> as FromSql>::accepts(ty)
    }
}

impl ToSql for Null {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>>
    where
        Self: Sized,
    {
        <Option<Vec<u8>> as ToSql>::to_sql(&None, ty, out)
    }

    fn accepts(ty: &Type) -> bool
    where
        Self: Sized,
    {
        <Option<Vec<u8>> as ToSql>::accepts(ty)
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        <Option<Vec<u8>> as ToSql>::to_sql_checked(&None, ty, out)
    }
}
