use crate::model::{Asn, Error, Model, Size};
use crate::parser::Token;
use std::convert::TryFrom;
use std::fmt::{Debug, Display};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct BitString<T: Display + Debug + Clone = usize> {
    pub size: Size<T>,
    pub constants: Vec<(String, u64)>,
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for BitString {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        let constants =
            Model::<Asn>::maybe_read_constants(iter, Model::<Asn>::constant_u64_parser)?;
        let size = Model::<Asn>::maybe_read_size(iter)?;
        Ok(Self { size, constants })
    }
}
