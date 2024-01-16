use crate::model::err::Error;
use crate::model::parse::PeekableTokens;
use crate::parser::Token;
use std::convert::TryFrom;
use std::iter::Peekable;

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct InnerTypeConstraints {
    implicit_all_present: bool,
    entries: Vec<(String, Option<ValueConstraint>, Option<PresenceConstraint>)>,
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for InnerTypeConstraints {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        iter.next_text_eq_ignore_case_or_err("WITH")?;
        iter.next_text_eq_ignore_case_or_err("COMPONENTS")?;
        iter.next_separator_eq_or_err('{')?;

        let implicit_all_present = if iter.peek_is_separator_eq('.') {
            iter.next_if_separator_and_eq('.')?;
            iter.next_if_separator_and_eq('.')?;
            iter.next_if_separator_and_eq('.')?;
            if iter.peek_is_separator_eq(',') {
                iter.next_separator_eq_or_err(',')?;
            }
            true
        } else {
            false
        };

        let mut entries = Vec::default();

        while !iter.peek_is_separator_eq('}') {
            let name = iter.next_text_or_err()?;
            let vconstr = if iter.peek_is_separator_eq('(') {
                iter.next_separator_eq_or_err('(')?;
                let result = ValueConstraint::try_from(&mut *iter)?;
                iter.next_separator_eq_or_err(')')?;
                Some(result)
            } else {
                None
            };

            let pconstr = if iter.peek_or_err()?.is_text() {
                Some(PresenceConstraint::try_from(&mut *iter)?)
            } else {
                None
            };

            entries.push((name, vconstr, pconstr));

            if iter.peek_is_separator_eq(',') {
                iter.next_separator_eq_or_err(',')?;
            } else {
                break;
            }
        }

        iter.next_separator_eq_or_err('}')?;

        Ok(Self {
            implicit_all_present,
            entries,
        })
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct ValueConstraint(String);

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for ValueConstraint {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        let mut level = 0_usize;
        let mut string = String::default();

        // TODO this is a very stupid implementation to just collect all the text within the parenthesis
        while !(level == 0 && iter.peek_is_separator_eq(')')) {
            match iter.next_or_err()? {
                Token::Text(_location, text) => string.push_str(&text),
                Token::Separator(_location, separator) => {
                    match separator {
                        '(' => level += 1,
                        ')' => level -= 1, // cannot underflow because of while condition
                        _ => {}
                    }
                    string.push(separator);
                }
            }
        }

        Ok(Self(string))
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum PresenceConstraint {
    Present,
    Absent,
    Optional,
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for PresenceConstraint {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        Ok(match iter.next_or_err()? {
            t if t.eq_text_ignore_ascii_case("PRESENT") => PresenceConstraint::Present,
            t if t.eq_text_ignore_ascii_case("ABSENT") => PresenceConstraint::Absent,
            t if t.eq_text_ignore_ascii_case("OPTIONAL") => PresenceConstraint::Optional,
            t => return Err(Error::unexpected_token(t)),
        })
    }
}
