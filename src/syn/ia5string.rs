use crate::syn::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub struct Ia5String<C: Constraint = NoConstraint>(PhantomData<C>);

impl<C: Constraint> Default for Ia5String<C> {
    fn default() -> Self {
        Ia5String(Default::default())
    }
}

pub trait Constraint {
    const MIN: Option<u64> = None;
    const MAX: Option<u64> = None;
    const EXTENSIBLE: bool = false;
}

#[derive(Default)]
pub struct NoConstraint;
impl Constraint for NoConstraint {}

impl<C: Constraint> WritableType for Ia5String<C> {
    type Type = String;

    #[inline]
    fn write_value<W: Writer>(writer: &mut W, value: &Self::Type) -> Result<(), W::Error> {
        writer.write_ia5string::<C>(value.as_str())
    }
}

impl<C: Constraint> ReadableType for Ia5String<C> {
    type Type = String;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_ia5string::<C>()
    }
}
