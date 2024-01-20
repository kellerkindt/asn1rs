use crate::descriptor::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub use crate::descriptor::sequence::Constraint;

pub struct Set<T: Constraint>(PhantomData<T>);

impl<C: Constraint> WritableType for Set<C> {
    type Type = C;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        writer.write_set::<C, _>(|w| value.write_seq::<W>(w))
    }
}

impl<C: Constraint> ReadableType for Set<C>
where
    C: Sized,
{
    type Type = C;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_set::<C, Self::Type, _>(C::read_seq)
    }
}
