use crate::syn::BitVec;
use postgres::types::{FromSql, IsNull, ToSql, Type};
use std::error::Error;

impl FromSql for BitVec {
    fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let vec = <Vec<u8> as FromSql>::from_sql(ty, raw)?;
        Ok(BitVec::from_vec_with_trailing_bit_len(vec))
    }

    fn accepts(ty: &Type) -> bool {
        <Vec<u8> as FromSql>::accepts(ty)
    }
}

impl ToSql for BitVec {
    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<dyn Error + Sync + Send>>
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
        out: &mut Vec<u8>,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let vec = self.to_vec_with_trailing_bit_len();
        <Vec<u8> as ToSql>::to_sql_checked(&vec, ty, out)
    }
}
