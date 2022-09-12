use crate::model::lor::Error as ResolveError;
use crate::model::lor::{ResolveState, Resolver, TryResolve, Unresolved};
use crate::model::{Asn, Error, LitOrRef, Model, Size};
use crate::parser::Token;
use std::convert::TryFrom;
use std::fmt::{Debug, Display};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct BitString<T: Display + Debug + Clone = usize> {
    pub size: Size<T>,
    pub constants: Vec<(String, u64)>,
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>>
    for BitString<<Unresolved as ResolveState>::SizeType>
{
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        let constants = Model::<Asn<Unresolved>>::maybe_read_constants(
            iter,
            Model::<Asn<Unresolved>>::constant_u64_parser,
        )?;
        let size = Model::<Asn<Unresolved>>::maybe_read_size(iter)?;
        Ok(Self { size, constants })
    }
}

impl TryResolve<usize, BitString<usize>> for BitString<LitOrRef<usize>> {
    fn try_resolve(
        &self,
        resolver: &impl Resolver<usize>,
    ) -> Result<BitString<usize>, ResolveError> {
        Ok(BitString {
            size: self.size.try_resolve(resolver)?,
            constants: self.constants.clone(),
        })
    }
}
