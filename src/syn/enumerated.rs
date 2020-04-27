use crate::syn::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub struct Enumerated<C: Constraint>(PhantomData<C>);

impl<C: Constraint> Default for Enumerated<C> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub trait Constraint: Sized {
    const NAME: &'static str;
    const STD_VARIANTS: usize;
    const EXTENSIBLE: bool = false;

    fn choice_index(&self) -> usize;

    fn from_choice_index(index: usize) -> Self;
}

impl<C: Constraint> WritableType for Enumerated<C> {
    type Type = C;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        writer.write_enumerated(value)
    }
}

impl<C: Constraint> ReadableType for Enumerated<C> {
    type Type = C;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_enumerated::<Self::Type>()
    }
}
