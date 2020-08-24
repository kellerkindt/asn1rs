use crate::syn::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub struct Integer<T: Copy = u64, C: Constraint<T> = NoConstraint>(PhantomData<T>, PhantomData<C>);

impl<T: Copy, C: Constraint<T>> Default for Integer<T, C> {
    fn default() -> Self {
        Integer(Default::default(), Default::default())
    }
}

pub trait Constraint<T: Copy> {
    const MIN: Option<T> = None;
    const MAX: Option<T> = None;
    const EXTENSIBLE: bool = false;
}

#[derive(Default)]
pub struct NoConstraint;

impl<T: Copy> Constraint<T> for NoConstraint {}

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
