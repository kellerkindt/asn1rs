use crate::descriptor::{ReadableType, Reader, WritableType, Writer};
use asn1rs_model::asn::Tag;
use core::marker::PhantomData;

pub struct Boolean<C: Constraint = NoConstraint>(PhantomData<C>);

pub trait Constraint: super::common::Constraint {}

#[derive(Default)]
pub struct NoConstraint;
impl super::common::Constraint for NoConstraint {
    const TAG: Tag = Tag::DEFAULT_BOOLEAN;
}
impl Constraint for NoConstraint {}

impl<C: Constraint> WritableType for Boolean<C> {
    type Type = bool;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        writer.write_boolean::<C>(*value)
    }
}

impl<C: Constraint> ReadableType for Boolean<C> {
    type Type = bool;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_boolean::<C>()
    }
}
