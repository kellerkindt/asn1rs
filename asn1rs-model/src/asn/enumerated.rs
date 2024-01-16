use crate::asn::peekable::PeekableTokens;
use crate::parse::Error;
use crate::parse::Token;
use std::convert::TryFrom;
use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct Enumerated {
    variants: Vec<EnumeratedVariant>,
    extension_after: Option<usize>,
}

impl From<Vec<EnumeratedVariant>> for Enumerated {
    fn from(variants: Vec<EnumeratedVariant>) -> Self {
        Self {
            variants,
            extension_after: None,
        }
    }
}

impl Enumerated {
    pub fn from_variants(variants: impl Into<Vec<EnumeratedVariant>>) -> Self {
        Self {
            variants: variants.into(),
            extension_after: None,
        }
    }

    pub fn from_names<I: ToString>(variants: impl Iterator<Item = I>) -> Self {
        Self {
            variants: variants.map(EnumeratedVariant::from_name).collect(),
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

    pub fn variants(&self) -> impl Iterator<Item = &EnumeratedVariant> {
        self.variants.iter()
    }

    pub fn is_extensible(&self) -> bool {
        self.extension_after.is_some()
    }

    pub fn extension_after_index(&self) -> Option<usize> {
        self.extension_after
    }
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for Enumerated {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        iter.next_separator_eq_or_err('{')?;
        let mut enumerated = Self {
            variants: Vec::new(),
            extension_after: None,
        };

        loop {
            if let Ok(extension_marker) = iter.next_if_separator_and_eq('.') {
                if enumerated.variants.is_empty() || enumerated.extension_after.is_some() {
                    return Err(Error::invalid_position_for_extension_marker(
                        extension_marker,
                    ));
                } else {
                    iter.next_separator_eq_or_err('.')?;
                    iter.next_separator_eq_or_err('.')?;
                    enumerated.extension_after = Some(enumerated.variants.len() - 1);
                    loop_ctrl_separator!(iter.next_or_err()?);
                }
            } else {
                let variant_name = iter.next_text_or_err()?;
                let token = iter.next_or_err()?;

                if token.eq_separator(',') || token.eq_separator('}') {
                    enumerated
                        .variants
                        .push(EnumeratedVariant::from_name(variant_name));
                    loop_ctrl_separator!(token);
                } else if token.eq_separator('(') {
                    let token = iter.next_or_err()?;
                    let number = token
                        .text()
                        .and_then(|t| t.parse::<usize>().ok())
                        .ok_or_else(|| Error::invalid_number_for_enum_variant(token))?;
                    iter.next_separator_eq_or_err(')')?;
                    enumerated
                        .variants
                        .push(EnumeratedVariant::from_name_number(variant_name, number));
                    loop_ctrl_separator!(iter.next_or_err()?);
                } else {
                    loop_ctrl_separator!(token);
                }
            }
        }

        Ok(enumerated)
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct EnumeratedVariant {
    pub(crate) name: String,
    pub(crate) number: Option<usize>,
}

#[cfg(test)]
impl<S: ToString> From<S> for EnumeratedVariant {
    fn from(s: S) -> Self {
        EnumeratedVariant::from_name(s)
    }
}

impl EnumeratedVariant {
    pub fn from_name<I: ToString>(name: I) -> Self {
        Self {
            name: name.to_string(),
            number: None,
        }
    }

    pub fn from_name_number<I: ToString>(name: I, number: usize) -> Self {
        Self {
            name: name.to_string(),
            number: Some(number),
        }
    }

    pub const fn with_number(self, number: usize) -> Self {
        self.with_number_opt(Some(number))
    }

    pub const fn with_number_opt(mut self, number: Option<usize>) -> Self {
        self.number = number;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn number(&self) -> Option<usize> {
        self.number
    }
}
