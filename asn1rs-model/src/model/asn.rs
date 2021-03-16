use crate::model::lor::{ResolveState, Resolved};
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

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Type<RS: ResolveState = Resolved> {
    Boolean,
    Integer(Integer<RS::RangeType>),
    String(Size<RS::SizeType>, Charset),
    OctetString(Size<RS::SizeType>),
    BitString(BitString<RS::SizeType>),

    Optional(Box<Type<RS>>),

    Sequence(ComponentTypeList),
    SequenceOf(Box<Type<RS>>, Size<RS::SizeType>),
    Set(ComponentTypeList),
    SetOf(Box<Type<RS>>, Size<RS::SizeType>),
    Enumerated(Enumerated),
    Choice(Choice),
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

    pub fn choice_from_variants(variants: Vec<ChoiceVariant>) -> Self {
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
