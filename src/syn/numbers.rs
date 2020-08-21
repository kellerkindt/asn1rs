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
    ( $read_int_max_fn:ident, $write_int_max_fn:ident, $($T:ident),+ ) => {$(

        impl<C: Constraint<$T>> WritableType for Integer<$T, C> {
            type Type = $T;

            #[inline]
            fn write_value<W: Writer>(
                writer: &mut W,
                value: &Self::Type,
            ) -> Result<(), <W as Writer>::Error> {
                let value = *value;
                if C::MIN.is_none() && C::MAX.is_none() {
                    writer.$write_int_max_fn(value as _)
                } else {
                    writer.write_int(
                        value as _,
                        (
                            C::MIN.map(|m| m as _).unwrap_or(0),
                            C::MAX.map(|m| m as _).unwrap_or_else(i64::max_value),
                        ),
                    )
                }
            }
        }

        impl<C: Constraint<$T>> ReadableType for Integer<$T, C> {
            type Type = $T;

            #[inline]
            fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
                if C::MIN.is_none() && C::MAX.is_none() {
                    Ok(reader.$read_int_max_fn()? as $T)
                } else {
                    Ok(reader
                        .read_int((
                            C::MIN.map(|m| m as _).unwrap_or(0),
                            C::MAX.map(|m| m as _).unwrap_or_else(i64::max_value),
                        ))? as $T
                    )
                }
            }
        }
     )*
    }
}

// don't ask me why u64 is in the signed section... but otherwise tests (with sample code provided
// by the asn playground) will fail
read_write!(
    read_int_max_signed,
    write_int_max_signed,
    i8,
    i16,
    i32,
    i64,
    u64
);
read_write!(read_int_max_unsigned, write_int_max_unsigned, u8, u16, u32);

/*
impl<C: Constraint<u64>> WritableType for Integer<u64, C> {
    type Type = u64;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        let value = *value;
        if C::MIN.is_none() && C::MAX.is_none() {
            writer.write_int_max_unsigned(value)
        } else {
            let value = i64::try_from(value).unwrap();
            writer.write_int(
                value,
                (
                    C::MIN.map(|v| i64::try_from(v).unwrap()).unwrap_or(0),
                    C::MAX
                        .map(|v| i64::try_from(v).unwrap())
                        .unwrap_or_else(i64::max_value),
                ),
            )
        }
    }
}

impl<C: Constraint<u64>> ReadableType for Integer<u64, C> {
    type Type = u64;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        if C::MIN.is_none() && C::MAX.is_none() {
            Ok(reader.read_int_max_unsigned()? as _)
        } else {
            Ok(reader.read_int((
                C::MIN.map(|v| i64::try_from(v).unwrap()).unwrap_or(0),
                C::MAX
                    .map(|v| i64::try_from(v).unwrap())
                    .unwrap_or_else(i64::max_value),
            ))? as u64)
        }
    }
}
*/
