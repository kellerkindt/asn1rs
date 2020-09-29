use crate::syn::bitstring::BitVec;
use bytes::BytesMut;
use std::error::Error;
use tokio_postgres::types::{FromSql, IsNull, ToSql, Type};

impl ToSql for BitVec {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>>
    where
        Self: Sized,
    {
        let vec = self.to_vec_with_trailing_bit_len();
        <Vec<u8> as ToSql>::to_sql(&vec, ty, out)
    }

    fn accepts(ty: &Type) -> bool
    where
        Self: Sized,
    {
        <Vec<u8> as ToSql>::accepts(ty)
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let vec = self.to_vec_with_trailing_bit_len();
        <Vec<u8> as ToSql>::to_sql_checked(&vec, ty, out)
    }
}

impl<'a> FromSql<'a> for BitVec {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let vec = <Vec<u8> as FromSql<'a>>::from_sql(ty, raw)?;
        Ok(BitVec::from_vec_with_trailing_bit_len(vec))
    }

    fn accepts(ty: &Type) -> bool {
        <Vec<u8> as FromSql<'a>>::accepts(ty)
    }
}
