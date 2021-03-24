use crate::model::lor::{Error as ResolveError, ResolveState, Resolved, Resolver, Unresolved};
use crate::model::{Asn, Error, Model, PeekableTokens, Tag, TagProperty, Type};
use crate::parser::Token;
use std::convert::TryFrom;

use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Choice<RS: ResolveState = Resolved> {
    variants: Vec<ChoiceVariant<RS>>,
    extension_after: Option<usize>,
}

impl<RS: ResolveState> From<Vec<ChoiceVariant<RS>>> for Choice<RS> {
    fn from(variants: Vec<ChoiceVariant<RS>>) -> Self {
        Self {
            variants,
            extension_after: None,
        }
    }
}

impl<RS: ResolveState> Choice<RS> {
    pub fn from_variants(variants: impl Iterator<Item = ChoiceVariant<RS>>) -> Self {
        Self {
            variants: variants.collect(),
            extension_after: None,
        }
    }

    pub fn with_extension_after(mut self, extension_after: usize) -> Self {
        self.extension_after = Some(extension_after);
        self
    }

    pub fn with_maybe_extension_after(mut self, extension_after: Option<usize>) -> Self {
        self.extension_after = extension_after;
        self
    }

    pub fn len(&self) -> usize {
        self.variants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    pub fn variants(&self) -> impl Iterator<Item = &ChoiceVariant<RS>> {
        self.variants.iter()
    }

    pub fn is_extensible(&self) -> bool {
        self.extension_after.is_some()
    }

    pub fn extension_after_index(&self) -> Option<usize> {
        self.extension_after
    }
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for Choice<Unresolved> {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        iter.next_separator_eq_or_err('{')?;
        let mut choice = Choice {
            variants: Vec::new(),
            extension_after: None,
        };

        loop {
            if let Ok(extension_marker) = iter.next_if_separator_and_eq('.') {
                if choice.variants.is_empty() || choice.extension_after.is_some() {
                    return Err(Error::invalid_position_for_extension_marker(
                        extension_marker,
                    ));
                } else {
                    iter.next_separator_eq_or_err('.')?;
                    iter.next_separator_eq_or_err('.')?;
                    choice.extension_after = Some(choice.variants.len() - 1);
                }
            } else {
                let name = iter.next_text_or_err()?;
                let (token, tag) = Model::<Asn<Unresolved>>::next_with_opt_tag(iter)?;
                let r#type = Model::<Asn<Unresolved>>::read_role_given_text(
                    iter,
                    token.into_text_or_else(Error::no_text)?,
                )?;
                choice.variants.push(ChoiceVariant { name, tag, r#type });
            }

            loop_ctrl_separator!(iter.next_or_err()?);
        }

        Ok(choice)
    }
}

impl Choice<Unresolved> {
    pub fn try_resolve<
        R: Resolver<<Resolved as ResolveState>::SizeType>
            + Resolver<<Resolved as ResolveState>::RangeType>,
    >(
        &self,
        resolver: &R,
    ) -> Result<Choice<Resolved>, ResolveError> {
        Ok(Choice {
            variants: self
                .variants
                .iter()
                .map(|v| v.try_resolve(resolver))
                .collect::<Result<Vec<_>, _>>()?,
            extension_after: self.extension_after,
        })
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ChoiceVariant<RS: ResolveState = Resolved> {
    pub name: String,
    pub tag: Option<Tag>,
    pub r#type: Type<RS>,
}

impl<RS: ResolveState> ChoiceVariant<RS> {
    #[cfg(test)]
    pub fn name_type<I: ToString>(name: I, r#type: Type<RS>) -> Self {
        ChoiceVariant {
            name: name.to_string(),
            tag: None,
            r#type,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn r#type(&self) -> &Type<RS> {
        &self.r#type
    }
}

impl<RS: ResolveState> TagProperty for ChoiceVariant<RS> {
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

impl ChoiceVariant<Unresolved> {
    pub fn try_resolve<
        R: Resolver<<Resolved as ResolveState>::SizeType>
            + Resolver<<Resolved as ResolveState>::RangeType>,
    >(
        &self,
        resolver: &R,
    ) -> Result<ChoiceVariant<Resolved>, ResolveError> {
        Ok(ChoiceVariant {
            name: self.name.clone(),
            tag: self.tag,
            r#type: self.r#type.try_resolve(resolver)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tag::tests::test_property;

    #[test]
    pub fn test_tag_property_choice_variant() {
        test_property(ChoiceVariant::<Resolved>::name_type(
            "VariantName".to_string(),
            Type::Boolean,
        ));
    }
}
