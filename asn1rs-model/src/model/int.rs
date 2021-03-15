use crate::model::{Asn, Error, Model, Range};
use crate::parser::Token;
use std::convert::TryFrom;
use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Integer {
    pub range: Range<Option<i64>>,
    pub constants: Vec<(String, i64)>,
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for Integer {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        let constants =
            Model::<Asn>::maybe_read_constants(iter, Model::<Asn>::constant_i64_parser)?;
        let range = Model::<Asn>::read_number_range(iter)?;
        Ok(Self { range, constants })
    }
}
