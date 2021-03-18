use crate::model::lor::Error as ResolveError;
use crate::model::lor::{ResolveState, Resolver, TryResolve, Unresolved};
use crate::model::{Asn, Error, LitOrRef, Model, PeekableTokens, Range};
use crate::parser::Token;
use std::convert::TryFrom;
use std::fmt::{Debug, Display};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Integer<T: Display + Debug + Clone = i64> {
    pub range: Range<Option<T>>,
    pub constants: Vec<(String, i64)>,
}

impl<T: Display + Debug + Clone> Default for Integer<T> {
    fn default() -> Self {
        Self {
            range: Range::none(),
            constants: Vec::default(),
        }
    }
}

impl<T: Display + Debug + Clone> Integer<T> {
    pub fn with_range(range: Range<Option<T>>) -> Self {
        Self {
            range,
            constants: Vec::default(),
        }
    }
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>>
    for Integer<<Unresolved as ResolveState>::RangeType>
{
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        let constants =
            Model::<Asn>::maybe_read_constants(iter, Model::<Asn>::constant_i64_parser)?;
        let range = if iter.next_is_separator_and_eq('(') {
            let start = iter.next_or_err()?;
            iter.next_separator_eq_or_err('.')?;
            iter.next_separator_eq_or_err('.')?;
            let end = iter.next_or_err()?;
            let extensible = if iter.next_is_separator_and_eq(',') {
                iter.next_separator_eq_or_err('.')?;
                iter.next_separator_eq_or_err('.')?;
                iter.next_separator_eq_or_err('.')?;
                true
            } else {
                false
            };
            iter.next_separator_eq_or_err(')')?;
            let start = start
                .text()
                .filter(|txt| !txt.eq_ignore_ascii_case("MIN"))
                .map(|t| match t.parse::<i64>() {
                    Ok(lit) => LitOrRef::Lit(lit),
                    Err(_) => LitOrRef::Ref(t.to_string()),
                });

            let end = end
                .text()
                .filter(|txt| !txt.eq_ignore_ascii_case("MAX"))
                .map(|t| match t.parse::<i64>() {
                    Ok(lit) => LitOrRef::Lit(lit),
                    Err(_) => LitOrRef::Ref(t.to_string()),
                });

            match (start, end) {
                (Some(LitOrRef::Lit(0)), None) | (None, Some(LitOrRef::Lit(i64::MAX))) => {
                    Range(None, None, extensible)
                }
                (start, end) => Range(start, end, extensible),
            }
        } else {
            Range(None, None, false)
        };
        Ok(Self { range, constants })
    }
}

impl TryResolve<i64, Integer<i64>> for Integer<LitOrRef<i64>> {
    fn try_resolve(&self, resolver: &impl Resolver<i64>) -> Result<Integer<i64>, ResolveError> {
        Ok(Integer {
            range: Range(
                self.range
                    .0
                    .as_ref()
                    .map(|lor| resolver.resolve(&lor))
                    .transpose()?,
                self.range
                    .1
                    .as_ref()
                    .map(|lor| resolver.resolve(&lor))
                    .transpose()?,
                self.range.2,
            ),
            //.reconsider_constraints(),
            constants: self.constants.clone(),
        })
    }
}
