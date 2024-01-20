use crate::descriptor::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub use crate::descriptor::sequenceof::Constraint;
pub use crate::descriptor::sequenceof::NoConstraint;

pub struct SetOf<T, C: Constraint = NoConstraint>(PhantomData<T>, PhantomData<C>);

impl<T: WritableType, C: Constraint> WritableType for SetOf<T, C> {
    type Type = Vec<T::Type>;

    #[inline]
    fn write_value<W: Writer>(writer: &mut W, value: &Self::Type) -> Result<(), W::Error> {
        writer.write_set_of::<C, T>(value.as_slice())
    }
}

impl<T: ReadableType, C: Constraint> ReadableType for SetOf<T, C> {
    type Type = Vec<T::Type>;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_set_of::<C, T>()
    }
}
