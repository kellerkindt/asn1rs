use crate::model::{Asn, Error, Model, PeekableTokens, Tag, TagProperty, Type};
use crate::parser::Token;
use std::convert::TryFrom;
use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Choice {
    variants: Vec<ChoiceVariant>,
    extension_after: Option<usize>,
}

impl From<Vec<ChoiceVariant>> for Choice {
    fn from(variants: Vec<ChoiceVariant>) -> Self {
        Self {
            variants,
            extension_after: None,
        }
    }
}

impl Choice {
    pub fn from_variants(variants: impl Iterator<Item = ChoiceVariant>) -> Self {
        Self {
            variants: variants.collect(),
            extension_after: None,
        }
    }

    pub const fn with_extension_after(mut self, extension_after: usize) -> Self {
        self.extension_after = Some(extension_after);
        self
    }

    pub const fn with_maybe_extension_after(mut self, extension_after: Option<usize>) -> Self {
        self.extension_after = extension_after;
        self
    }

    pub fn len(&self) -> usize {
        self.variants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    pub fn variants(&self) -> impl Iterator<Item = &ChoiceVariant> {
        self.variants.iter()
    }

    pub fn is_extensible(&self) -> bool {
        self.extension_after.is_some()
    }

    pub fn extension_after_index(&self) -> Option<usize> {
        self.extension_after
    }
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for Choice {
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
                let (token, tag) = Model::<Asn>::next_with_opt_tag(iter)?;
                let r#type = Model::<Asn>::read_role_given_text(
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

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ChoiceVariant {
    pub name: String,
    pub tag: Option<Tag>,
    pub r#type: Type,
}

impl ChoiceVariant {
    #[cfg(test)]
    pub fn name_type<I: ToString>(name: I, r#type: Type) -> Self {
        ChoiceVariant {
            name: name.to_string(),
            tag: None,
            r#type,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn r#type(&self) -> &Type {
        &self.r#type
    }
}

impl TagProperty for ChoiceVariant {
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
