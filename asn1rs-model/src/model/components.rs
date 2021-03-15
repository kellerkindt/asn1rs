use crate::model::{Asn, Error, Field, Model, PeekableTokens};
use crate::parser::Token;
use std::convert::TryFrom;
use std::iter::Peekable;

/// ITU-T X.680 | ISO/IEC 8824-1:2015, Annex L
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ComponentTypeList {
    pub fields: Vec<Field<Asn>>,
    pub extension_after: Option<usize>,
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for ComponentTypeList {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        iter.next_separator_eq_or_err('{')?;
        let mut sequence = Self {
            fields: Vec::default(),
            extension_after: None,
        };

        loop {
            let continues = if iter.next_is_separator_and_eq('}') {
                false
            } else if iter.next_is_separator_and_eq('.') {
                iter.next_separator_eq_or_err('.')?;
                iter.next_separator_eq_or_err('.')?;
                let field_len = sequence.fields.len();
                sequence.extension_after = Some(field_len.saturating_sub(1));

                match iter.next_or_err()? {
                    token if token.eq_separator(',') => true,
                    token if token.eq_separator('}') => false,
                    token => return Err(Error::unexpected_token(token)),
                }
            } else {
                let (field, continues) = Model::<Asn>::read_field(iter)?;
                sequence.fields.push(field);
                continues
            };

            if !continues {
                break;
            }
        }

        Ok(sequence)
    }
}
