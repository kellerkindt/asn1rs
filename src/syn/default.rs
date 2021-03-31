use crate::syn::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;
use std::fmt::Debug;

pub struct DefaultValue<T, C: Constraint>(PhantomData<T>, PhantomData<C>);

pub trait Constraint: super::common::Constraint {
    type Owned;
    type Borrowed: PartialEq<Self::Owned>
        + ToOwned<Owned = <Self as Constraint>::Owned>
        + Debug
        + 'static
        + ?Sized;

    const DEFAULT_VALUE: &'static Self::Borrowed;
}

impl<T: WritableType, C: Constraint<Owned = T::Type>> WritableType for DefaultValue<T, C> {
    type Type = T::Type;

    #[inline]
    fn write_value<W: Writer>(writer: &mut W, value: &Self::Type) -> Result<(), W::Error> {
        writer.write_default::<C, T>(value)
    }
}

impl<T: ReadableType, C: Constraint<Owned = T::Type>> ReadableType for DefaultValue<T, C> {
    type Type = T::Type;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_default::<C, T>()
    }
}
