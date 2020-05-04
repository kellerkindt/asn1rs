use crate::syn::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub struct Boolean<C: Constraint = NoConstraint>(PhantomData<C>);

impl<C: Constraint> Default for Boolean<C> {
    fn default() -> Self {
        Boolean(Default::default())
    }
}

pub trait Constraint {}

#[derive(Default)]
pub struct NoConstraint;
impl Constraint for NoConstraint {}

impl<C: Constraint> WritableType for Boolean<C> {
    type Type = bool;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        writer.write_boolean::<C>(*value)
    }
}

impl<C: Constraint> ReadableType for Boolean<C> {
    type Type = bool;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_boolean::<C>()
    }
}
