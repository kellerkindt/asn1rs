use crate::asn::peekable::PeekableTokens;
use crate::asn::{Asn, Type};
use crate::model::{Field, Model};
use crate::parse::Error;
use crate::parse::Token;
use crate::resolve::{Error as ResolveError, Resolved, Resolver};
use crate::resolve::{ResolveState, Unresolved};
use std::convert::TryFrom;
use std::iter::Peekable;

/// ITU-T X.680 | ISO/IEC 8824-1:2015, Annex L
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct ComponentTypeList<RS: ResolveState = Unresolved> {
    pub fields: Vec<Field<Asn<RS>>>,
    pub extension_after: Option<usize>,
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for ComponentTypeList<Unresolved> {
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
                let (field, continues) = Model::<Asn<Unresolved>>::read_field(iter)?;
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

impl ComponentTypeList<Unresolved> {
    pub fn try_resolve<
        R: Resolver<<Resolved as ResolveState>::SizeType>
            + Resolver<<Resolved as ResolveState>::RangeType>
            + Resolver<<Resolved as ResolveState>::ConstType>
            + Resolver<Type<Unresolved>>,
    >(
        &self,
        resolver: &R,
    ) -> Result<ComponentTypeList<Resolved>, ResolveError> {
        Ok(ComponentTypeList {
            fields: self
                .fields
                .iter()
                .map(|f| f.try_resolve(resolver))
                .collect::<Result<Vec<_>, _>>()?,
            extension_after: self.extension_after,
        })
    }
}
