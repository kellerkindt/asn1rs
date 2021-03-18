use crate::model::lor::{Error as ResolveError, TryResolve, Unresolved};
use crate::model::lor::{ResolveState, Resolved, Resolver};
use crate::model::{
    BitString, Charset, Choice, ChoiceVariant, ComponentTypeList, Enumerated, Field, Integer,
    Range, Size, Tag, TagProperty,
};
use std::fmt::Debug;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Asn<RS: ResolveState = Resolved> {
    pub tag: Option<Tag>,
    pub r#type: Type<RS>,
}

impl<RS: ResolveState> Asn<RS> {
    pub fn make_optional(&mut self) {
        let optional = self.r#type.clone().optional();
        self.r#type = optional;
    }

    pub fn opt_tagged(tag: Option<Tag>, r#type: Type<RS>) -> Self {
        Self { tag, r#type }
    }

    pub fn untagged(r#type: Type<RS>) -> Self {
        Self::opt_tagged(None, r#type)
    }

    pub fn tagged(tag: Tag, r#type: Type<RS>) -> Self {
        Self::opt_tagged(Some(tag), r#type)
    }
}

impl From<Type> for Asn {
    fn from(r#type: Type) -> Self {
        Self::untagged(r#type)
    }
}

impl TagProperty for Asn {
    fn tag(&self) -> Option<Tag> {
        self.tag
    }

    fn set_tag(&mut self, tag: Tag) {
        self.tag = Some(tag)
    }

    fn reset_tag(&mut self) {
        self.tag = None
    }
}

impl Asn<Unresolved> {
    pub fn try_resolve<
        R: Resolver<<Resolved as ResolveState>::SizeType>
            + Resolver<<Resolved as ResolveState>::RangeType>,
    >(
        &self,
        resolver: &R,
    ) -> Result<Asn<Resolved>, ResolveError> {
        Ok(Asn {
            tag: self.tag,
            r#type: self.r#type.try_resolve(resolver)?,
        })
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Type<RS: ResolveState = Resolved> {
    Boolean,
    Integer(Integer<RS::RangeType>),
    String(Size<RS::SizeType>, Charset),
    OctetString(Size<RS::SizeType>),
    BitString(BitString<RS::SizeType>),

    Optional(Box<Type<RS>>),

    Sequence(ComponentTypeList<RS>),
    SequenceOf(Box<Type<RS>>, Size<RS::SizeType>),
    Set(ComponentTypeList<RS>),
    SetOf(Box<Type<RS>>, Size<RS::SizeType>),
    Enumerated(Enumerated),
    Choice(Choice<RS>),
    TypeReference(String, Option<Tag>),
}

impl Type {
    pub fn unconstrained_integer() -> Self {
        Self::integer_with_range_opt(Range::none())
    }

    pub const fn sequence_from_fields(fields: Vec<Field<Asn>>) -> Self {
        Self::Sequence(ComponentTypeList {
            fields,
            extension_after: None,
        })
    }
}

impl<RS: ResolveState> Type<RS> {
    pub fn unconstrained_utf8string() -> Self {
        Self::String(Size::Any, Charset::Utf8)
    }

    pub fn unconstrained_octetstring() -> Self {
        Self::OctetString(Size::Any)
    }

    pub fn integer_with_range(range: Range<Option<RS::RangeType>>) -> Self {
        Self::Integer(Integer {
            range,
            constants: Vec::new(),
        })
    }

    pub fn integer_with_range_opt(range: Range<Option<RS::RangeType>>) -> Self {
        Self::Integer(Integer {
            range,
            constants: Vec::new(),
        })
    }

    pub fn bit_vec_with_size(size: Size<RS::SizeType>) -> Self {
        Self::BitString(BitString {
            size,
            constants: Vec::new(),
        })
    }

    pub fn choice_from_variants(variants: Vec<ChoiceVariant<RS>>) -> Self {
        Self::Choice(Choice::from(variants))
    }

    pub fn optional(self) -> Self {
        Self::Optional(Box::new(self))
    }

    pub fn opt_tagged(self, tag: Option<Tag>) -> Asn<RS> {
        Asn::opt_tagged(tag, self)
    }

    pub fn tagged(self, tag: Tag) -> Asn<RS> {
        Asn::tagged(tag, self)
    }

    pub fn untagged(self) -> Asn<RS> {
        Asn::untagged(self)
    }

    pub fn no_optional_mut(&mut self) -> &mut Self {
        if let Self::Optional(inner) = self {
            inner.no_optional_mut()
        } else {
            self
        }
    }
}

impl Type<Unresolved> {
    pub fn try_resolve<
        R: Resolver<<Resolved as ResolveState>::SizeType>
            + Resolver<<Resolved as ResolveState>::RangeType>,
    >(
        &self,
        resolver: &R,
    ) -> Result<Type<Resolved>, ResolveError> {
        Ok(match self {
            Type::Boolean => Type::Boolean,
            Type::Integer(integer) => Type::Integer(integer.try_resolve(resolver)?),
            Type::String(size, charset) => Type::String(size.try_resolve(resolver)?, *charset),
            Type::OctetString(size) => Type::OctetString(size.try_resolve(resolver)?),
            Type::BitString(string) => Type::BitString(string.try_resolve(resolver)?),
            Type::Optional(inner) => Type::Optional(Box::new(inner.try_resolve(resolver)?)),
            Type::Sequence(seq) => Type::Sequence(seq.try_resolve(resolver)?),
            Type::SequenceOf(inner, size) => Type::SequenceOf(
                Box::new(inner.try_resolve(resolver)?),
                size.try_resolve(resolver)?,
            ),
            Type::Set(set) => Type::Set(set.try_resolve(resolver)?),
            Type::SetOf(inner, size) => Type::SetOf(
                Box::new(inner.try_resolve(resolver)?),
                size.try_resolve(resolver)?,
            ),
            Type::Enumerated(e) => Type::Enumerated(e.clone()),
            Type::Choice(c) => Type::Choice(c.try_resolve(resolver)?),
            Type::TypeReference(name, tag) => Type::TypeReference(name.clone(), *tag),
        })
    }
}
