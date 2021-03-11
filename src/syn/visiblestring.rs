use crate::model::Tag;
use crate::syn::{ReadableType, Reader, WritableType, Writer};
use core::marker::PhantomData;

pub struct VisibleString<C: Constraint = NoConstraint>(PhantomData<C>);

impl<C: Constraint> Default for VisibleString<C> {
    fn default() -> Self {
        VisibleString(Default::default())
    }
}

pub trait Constraint: super::common::Constraint {
    const MIN: Option<u64> = None;
    const MAX: Option<u64> = None;
    const EXTENSIBLE: bool = false;
}

#[derive(Default)]
pub struct NoConstraint;
impl super::common::Constraint for NoConstraint {
    const TAG: Tag = Tag::DEFAULT_VISIBLE_STRING;
}
impl Constraint for NoConstraint {}

impl<C: Constraint> WritableType for VisibleString<C> {
    type Type = String;

    #[inline]
    fn write_value<W: Writer>(writer: &mut W, value: &Self::Type) -> Result<(), W::Error> {
        writer.write_visible_string::<C>(value.as_str())
    }
}

impl<C: Constraint> ReadableType for VisibleString<C> {
    type Type = String;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_visible_string::<C>()
    }
}
