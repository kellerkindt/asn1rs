use crate::model::lor::{Error as ResolveError, TryResolve, Unresolved};
use crate::model::lor::{ResolveState, Resolved, Resolver};
use crate::model::{
    BitString, Charset, Choice, ChoiceVariant, ComponentTypeList, Enumerated, Field, Integer,
    LiteralValue, Range, Size, Tag, TagProperty, Target,
};
use std::fmt::Debug;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Asn<RS: ResolveState = Resolved> {
    pub tag: Option<Tag>,
    pub r#type: Type<RS>,
    pub default: Option<RS::ConstType>,
}

impl<RS: ResolveState> Target for Asn<RS> {
    type DefinitionType = Self;
    type ValueReferenceType = Self;
}

impl<RS: ResolveState> Asn<RS> {
    pub fn make_optional(&mut self) {
        let optional = self.r#type.clone().optional();
        self.r#type = optional;
    }

    pub fn set_default(&mut self, value: RS::ConstType) {
        self.default = Some(value);
    }

    pub fn opt_tagged(tag: Option<Tag>, r#type: Type<RS>) -> Self {
        Self {
            tag,
            r#type,
            default: None,
        }
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
            + Resolver<<Resolved as ResolveState>::RangeType>
            + Resolver<<Resolved as ResolveState>::ConstType>,
    >(
        &self,
        resolver: &R,
    ) -> Result<Asn<Resolved>, ResolveError> {
        Ok(Asn {
            tag: self.tag,
            r#type: self.r#type.try_resolve(resolver)?,
            default: self
                .default
                .as_ref()
                .map(|d| resolver.resolve(d))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Type<RS: ResolveState = Resolved> {
    /// ITU-T X.680 | ISO/IEC 8824-1, 18
    Boolean,
    /// ITU-T X.680 | ISO/IEC 8824-1, 19
    Integer(Integer<RS::RangeType>),
    String(Size<RS::SizeType>, Charset),
    /// ITU-T X.680 | ISO/IEC 8824-1, 23
    OctetString(Size<RS::SizeType>),
    /// ITU-T X.680 | ISO/IEC 8824-1, 22
    BitString(BitString<RS::SizeType>),

    Optional(Box<Type<RS>>),
    Default(Box<Type<RS>>, LiteralValue),

    /// ITU-T X.680 | ISO/IEC 8824-1, 25
    Sequence(ComponentTypeList<RS>),
    /// ITU-T X.680 | ISO/IEC 8824-1, 26
    SequenceOf(Box<Type<RS>>, Size<RS::SizeType>),
    /// ITU-T X.680 | ISO/IEC 8824-1, 27
    Set(ComponentTypeList<RS>),
    /// ITU-T X.680 | ISO/IEC 8824-1, 28
    SetOf(Box<Type<RS>>, Size<RS::SizeType>),
    /// ITU-T X.680 | ISO/IEC 8824-1, 20
    Enumerated(Enumerated),
    /// ITU-T X.680 | ISO/IEC 8824-1, 29
    Choice(Choice<RS>),

    /// ITU-T X.680 | ISO/IEC 8824-1, 16
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
            + Resolver<<Resolved as ResolveState>::RangeType>
            + Resolver<<Resolved as ResolveState>::ConstType>,
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
            Type::Default(inner, default) => {
                Type::Default(Box::new(inner.try_resolve(resolver)?), default.clone())
            }
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

impl LiteralValue {
    pub fn try_from_asn_str(asn: &str) -> Option<LiteralValue> {
        Some(match asn {
            bool if bool.eq_ignore_ascii_case("true") => LiteralValue::Boolean(true),
            bool if bool.eq_ignore_ascii_case("false") => LiteralValue::Boolean(false),
            slice if slice.starts_with('"') && slice.ends_with('"') => {
                LiteralValue::String(slice[1..slice.len() - 1].to_owned())
            }
            slice
                if slice.chars().all(|c| c.is_ascii_digit())
                    || (slice.starts_with('-')
                        && slice.len() > 1
                        && slice.chars().skip(1).all(|c| c.is_ascii_digit())) =>
            {
                LiteralValue::Integer(slice.parse().ok()?)
            }
            slice
                if slice.starts_with('\'') && (slice.ends_with("'h") || slice.ends_with("'H")) =>
            {
                let hex = &slice[1..slice.len() - 2];
                if hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    let mut vec = Vec::with_capacity((hex.len() + 1) / 2);
                    let offset = hex.len() % 2;

                    if offset > 0 {
                        let init = &hex[..offset];
                        vec.push(u8::from_str_radix(init, 16).ok()?);
                    }

                    for i in 0..hex.len() / 2 {
                        let position = offset + (2 * i);
                        vec.push(u8::from_str_radix(&hex[position..position + 2], 16).ok()?);
                    }

                    LiteralValue::OctetString(vec)
                } else {
                    return None;
                }
            }
            slice
                if slice.starts_with('\'') && (slice.ends_with("'b") || slice.ends_with("'B")) =>
            {
                let bits = &slice[1..slice.len() - 2];
                let mut vec = vec![0x00u8; (bits.len() + 7) / 8];

                for (i, bit) in bits.chars().rev().enumerate() {
                    if bit == '1' {
                        let target_index = vec.len() - 1 - (i / 8);
                        let value = 2u8.pow((i % 8) as u32);
                        vec[target_index] += value;
                    } else if bit != '0' {
                        return None;
                    }
                }

                LiteralValue::OctetString(vec)
            }

            _ => return None,
        })
    }
}
