use crate::syn::{ReadableType, Reader, WritableType, Writer};
use asn1rs_model::model::Tag;
use core::marker::PhantomData;

pub struct Integer<T: Number = u64, C: Constraint<T> = NoConstraint>(
    PhantomData<T>,
    PhantomData<C>,
);

pub trait Number: Copy {
    fn to_i64(self) -> i64;

    fn from_i64(value: i64) -> Self;
}

pub trait Constraint<T: Number>: super::common::Constraint {
    // TODO MIN-MAX into RANGE: Option<(T, T)>
    const MIN: Option<i64> = None;
    const MAX: Option<i64> = None;
    const MIN_T: Option<T> = None;
    const MAX_T: Option<T> = None;
    const EXTENSIBLE: bool = false;
}

#[derive(Default)]
pub struct NoConstraint;
impl super::common::Constraint for NoConstraint {
    const TAG: Tag = Tag::DEFAULT_INTEGER;
}
impl<T: Number> Constraint<T> for NoConstraint {}

impl<T: Number, C: Constraint<T>> WritableType for Integer<T, C> {
    type Type = T;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        writer.write_number::<T, C>(*value)
    }
}

impl<T: Number, C: Constraint<T>> ReadableType for Integer<T, C> {
    type Type = T;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_number::<T, C>()
    }
}

macro_rules! impl_number {
    ( $($T:ident),+ ) => {$(
        impl Number for $T {
            #[inline]
            fn to_i64(self) -> i64 {
                self as i64
            }

            #[inline]
            fn from_i64(value: i64) -> Self {
                value as $T
            }
        }
    )*}
}

impl_number!(u8, u16, u32, u64);
impl_number!(i8, i16, i32, i64);

/*
macro_rules! read_write {
    ( $($T:ident),+ ) => {$(

        impl<C: Constraint<$T>> WritableType for Integer<$T, C> {
            type Type = $T;

            #[inline]
            fn write_value<W: Writer>(
                writer: &mut W,
                value: &Self::Type,
            ) -> Result<(), <W as Writer>::Error> {
                paste! { writer.[<write_int_ $T>]::<C>(*value) }
            }
        }

        impl<C: Constraint<$T>> ReadableType for Integer<$T, C> {
            type Type = $T;

            #[inline]
            fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
                paste! { reader.[<read_int_ $T>]::<C>() }
            }
        }
     )*
    }
}

read_write!(i8, i16, i32, i64);
read_write!(u8, u16, u32, u64);
*/
