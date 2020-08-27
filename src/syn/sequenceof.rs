use crate::syn::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub struct SequenceOf<T, C: Constraint = NoConstraint>(PhantomData<T>, PhantomData<C>);

impl<C: Constraint> Default for SequenceOf<C> {
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}

pub trait Constraint {
    const MIN: Option<u64> = None;
    const MAX: Option<u64> = None;
}

#[derive(Default)]
pub struct NoConstraint;
impl Constraint for NoConstraint {}

impl<T: WritableType, C: Constraint> WritableType for SequenceOf<T, C> {
    type Type = Vec<T::Type>;

    #[inline]
    fn write_value<W: Writer>(writer: &mut W, value: &Self::Type) -> Result<(), W::Error> {
        writer.write_sequence_of::<C, T>(value.as_slice())
    }
}

impl<T: ReadableType, C: Constraint> ReadableType for SequenceOf<T, C> {
    type Type = Vec<T::Type>;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_sequence_of::<C, T>()
    }
}
