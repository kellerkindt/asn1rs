use crate::syn::{Readable, ReadableType, Reader, Writable, WritableType, Writer};
use core::marker::PhantomData;

pub struct Complex<V, T: Constraint>(PhantomData<T>, PhantomData<V>);

impl<V, T: Constraint> Default for Complex<V, T> {
    fn default() -> Self {
        Complex(Default::default(), Default::default())
    }
}

pub trait Constraint: super::common::Constraint {}

impl<V: Writable, C: Constraint> WritableType for Complex<V, C> {
    type Type = V;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        value.write(writer)
    }
}

impl<V: Readable, C: Constraint> ReadableType for Complex<V, C> {
    type Type = V;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        V::read(reader)
    }
}
