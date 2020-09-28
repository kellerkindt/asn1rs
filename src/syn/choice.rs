use crate::syn::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub struct Choice<C: Constraint>(PhantomData<C>);

impl<C: Constraint> Default for Choice<C> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub trait Constraint: super::common::Constraint + Sized {
    const NAME: &'static str;
    const VARIANT_COUNT: u64;
    const STD_VARIANT_COUNT: u64;
    const EXTENSIBLE: bool = false;

    fn to_choice_index(&self) -> u64;

    fn write_content<W: Writer>(&self, writer: &mut W) -> Result<(), W::Error>;

    fn read_content<R: Reader>(index: u64, reader: &mut R) -> Result<Option<Self>, R::Error>;
}

impl<C: Constraint> WritableType for Choice<C> {
    type Type = C;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        writer.write_choice(value)
    }
}

impl<C: Constraint> ReadableType for Choice<C> {
    type Type = C;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_choice::<Self::Type>()
    }
}
