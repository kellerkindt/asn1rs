use crate::syn::{ReadableType, Reader, WritableType, Writer};
use asn1rs_model::asn::Tag;
use core::marker::PhantomData;

pub struct NullT<C: Constraint = NoConstraint>(PhantomData<C>);

pub trait Constraint: super::common::Constraint {}

#[derive(Default)]
pub struct NoConstraint;
impl super::common::Constraint for NoConstraint {
    const TAG: Tag = Tag::DEFAULT_NULL;
}
impl Constraint for NoConstraint {}

impl<C: Constraint> WritableType for NullT<C> {
    type Type = Null;

    #[inline]
    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        writer.write_null::<C>(value)
    }
}

impl<C: Constraint> ReadableType for NullT<C> {
    type Type = Null;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_null::<C>()
    }
}

#[derive(Default, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Null;

impl From<()> for Null {
    fn from(_value: ()) -> Self {
        Null
    }
}

impl From<Null> for () {
    fn from(_value: Null) -> Self {}
}
