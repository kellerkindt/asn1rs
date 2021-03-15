use crate::model::{
    BitString, Charset, Choice, ChoiceVariant, ComponentTypeList, Enumerated, Field, Integer,
    Range, Size, Tag, TagProperty,
};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Asn {
    pub tag: Option<Tag>,
    pub r#type: Type,
}

impl Asn {
    pub fn make_optional(&mut self) {
        let optional = self.r#type.clone().optional();
        self.r#type = optional;
    }

    pub const fn opt_tagged(tag: Option<Tag>, r#type: Type) -> Self {
        Self { tag, r#type }
    }

    pub const fn untagged(r#type: Type) -> Self {
        Self::opt_tagged(None, r#type)
    }

    pub const fn tagged(tag: Tag, r#type: Type) -> Self {
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
pub enum Type {
    Boolean,
    Integer(Integer),
    String(Size, Charset),
    OctetString(Size),
    BitString(BitString),

    Optional(Box<Type>),

    Sequence(ComponentTypeList),
    SequenceOf(Box<Type>, Size),
    Set(ComponentTypeList),
    SetOf(Box<Type>, Size),
    Enumerated(Enumerated),
    Choice(Choice),
    TypeReference(String, Option<Tag>),
}

impl Type {
    pub const fn unconstrained_utf8string() -> Self {
        Self::String(Size::Any, Charset::Utf8)
    }

    pub const fn unconstrained_octetstring() -> Self {
        Self::OctetString(Size::Any)
    }

    pub fn unconstrained_integer() -> Self {
        Self::integer_with_range_opt(Range::none())
    }

    pub const fn integer_with_range(range: Range<Option<i64>>) -> Self {
        Self::Integer(Integer {
            range,
            constants: Vec::new(),
        })
    }

    pub const fn integer_with_range_opt(range: Range<Option<i64>>) -> Self {
        Self::Integer(Integer {
            range,
            constants: Vec::new(),
        })
    }

    pub const fn bit_vec_with_size(size: Size) -> Self {
        Self::BitString(BitString {
            size,
            constants: Vec::new(),
        })
    }

    pub const fn sequence_from_fields(fields: Vec<Field<Asn>>) -> Self {
        Self::Sequence(ComponentTypeList {
            fields,
            extension_after: None,
        })
    }

    pub fn choice_from_variants(variants: Vec<ChoiceVariant>) -> Self {
        Self::Choice(Choice::from(variants))
    }

    pub fn optional(self) -> Self {
        Self::Optional(Box::new(self))
    }

    pub const fn opt_tagged(self, tag: Option<Tag>) -> Asn {
        Asn::opt_tagged(tag, self)
    }

    pub const fn tagged(self, tag: Tag) -> Asn {
        Asn::tagged(tag, self)
    }

    pub const fn untagged(self) -> Asn {
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
