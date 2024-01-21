use crate::descriptor::{ReadableType, Reader, WritableType, Writer};
use asn1rs_model::asn::Tag;
use core::marker::PhantomData;

pub struct OctetString<C: Constraint = NoConstraint>(PhantomData<C>);

pub trait Constraint: super::common::Constraint {
    const MIN: Option<u64> = None;
    const MAX: Option<u64> = None;
    const EXTENSIBLE: bool = false;
}

#[derive(Default)]
pub struct NoConstraint;
impl super::common::Constraint for NoConstraint {
    const TAG: Tag = Tag::DEFAULT_OCTET_STRING;
}
impl Constraint for NoConstraint {}

impl<C: Constraint> WritableType for OctetString<C> {
    type Type = Vec<u8>;

    #[inline]
    fn write_value<W: Writer>(writer: &mut W, value: &Self::Type) -> Result<(), W::Error> {
        writer.write_octet_string::<C>(value.as_slice())
    }
}

impl<C: Constraint> ReadableType for OctetString<C> {
    type Type = Vec<u8>;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_octet_string::<C>()
    }
}