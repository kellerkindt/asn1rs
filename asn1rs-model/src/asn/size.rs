use crate::asn::peekable::PeekableTokens;
use crate::parse::Error;
use crate::parse::Token;
use crate::resolve::{Error as ResolveError, LitOrRef, Resolver, TryResolve};
use crate::resolve::{ResolveState, Unresolved};
use std::convert::TryFrom;
use std::fmt::{Debug, Display};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum Size<T: Display + Debug + Clone = usize> {
    Any,
    Fix(T, bool),
    Range(T, T, bool),
}

impl<T: Display + Debug + Clone> Size<T> {
    pub fn min(&self) -> Option<&T> {
        match self {
            Size::Any => None,
            Size::Fix(min, _) => Some(min),
            Size::Range(min, _, _) => Some(min),
        }
    }

    pub fn max(&self) -> Option<&T> {
        match self {
            Size::Any => None,
            Size::Fix(max, _) => Some(max),
            Size::Range(_, max, _) => Some(max),
        }
    }

    pub fn extensible(&self) -> bool {
        match self {
            Size::Any => false,
            Size::Fix(_, extensible) => *extensible,
            Size::Range(_, _, extensible) => *extensible,
        }
    }

    pub fn to_constraint_string(&self) -> Option<String> {
        match self {
            Size::Any => None,
            Size::Fix(min, extensible) => Some(format!(
                "size({}{})",
                min,
                if *extensible { ",..." } else { "" }
            )),
            Size::Range(min, max, extensible) => Some(format!(
                "size({}..{}{})",
                min,
                max,
                if *extensible { ",..." } else { "" }
            )),
        }
    }
}

impl Size<usize> {
    pub fn reconsider_constraints(self) -> Self {
        if let Self::Range(min, max, extensible) = self {
            if min == 0 && max == i64::MAX as usize && !extensible {
                Self::Any
            } else if min == max {
                Self::Fix(min, extensible)
            } else {
                self
            }
        } else {
            self
        }
    }
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>>
    for Size<<Unresolved as ResolveState>::SizeType>
{
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        iter.next_text_eq_ignore_case_or_err("SIZE")?;
        iter.next_separator_eq_or_err('(')?;

        let start = iter.next_or_err()?;
        let start = start
            .text()
            .filter(|txt| !txt.eq_ignore_ascii_case("MIN"))
            .map(|t| match t.parse::<usize>() {
                Ok(lit) => LitOrRef::Lit(lit),
                Err(_) => LitOrRef::Ref(t.to_string()),
            })
            .filter(|lor| LitOrRef::Lit(0).ne(lor));

        if !iter.peek_is_separator_eq('.') {
            match iter.next_or_err()? {
                t if t.eq_separator(')') => Ok(Size::Fix(start.unwrap_or_default(), false)),
                t if t.eq_separator(',') => {
                    iter.next_separator_eq_or_err('.')?;
                    iter.next_separator_eq_or_err('.')?;
                    iter.next_separator_eq_or_err('.')?;
                    iter.next_separator_eq_or_err(')')?;
                    Ok(Size::Fix(start.unwrap_or_default(), true))
                }
                t => Err(Error::unexpected_token(t)),
            }
        } else {
            const MAX: usize = i64::MAX as usize;

            iter.next_separator_eq_or_err('.')?;
            iter.next_separator_eq_or_err('.')?;
            let end = iter.next_or_err()?;
            let end = end
                .text()
                .filter(|txt| !txt.eq_ignore_ascii_case("MAX"))
                .map(|t| match t.parse::<usize>() {
                    Ok(lit) => LitOrRef::Lit(lit),
                    Err(_) => LitOrRef::Ref(t.to_string()),
                })
                .filter(|lor| LitOrRef::Lit(MAX).ne(lor));

            let any = matches!(
                (&start, &end),
                (None, None) | (Some(LitOrRef::Lit(0)), None) | (None, Some(LitOrRef::Lit(MAX)))
            );

            if any {
                iter.next_separator_eq_or_err(')')?;
                Ok(Size::Any)
            } else {
                let start = start.unwrap_or_default();
                let end = end.unwrap_or(LitOrRef::Lit(i64::MAX as usize));
                let extensible = if iter.next_separator_eq_or_err(',').is_ok() {
                    iter.next_separator_eq_or_err('.')?;
                    iter.next_separator_eq_or_err('.')?;
                    iter.next_separator_eq_or_err('.')?;
                    true
                } else {
                    false
                };
                iter.next_separator_eq_or_err(')')?;
                if start == end {
                    Ok(Size::Fix(start, extensible))
                } else {
                    Ok(Size::Range(start, end, extensible))
                }
            }
        }
    }
}

impl TryResolve<usize, Size<usize>> for Size<LitOrRef<usize>> {
    fn try_resolve(&self, resolver: &impl Resolver<usize>) -> Result<Size<usize>, ResolveError> {
        Ok(match self {
            Size::Any => Size::Any,
            Size::Fix(len, ext) => Size::Fix(resolver.resolve(len)?, *ext),
            Size::Range(min, max, ext) => {
                Size::Range(resolver.resolve(min)?, resolver.resolve(max)?, *ext)
            }
        }
        .reconsider_constraints())
    }
}
