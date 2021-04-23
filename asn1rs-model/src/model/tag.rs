use crate::model::{Error, PeekableTokens};
use crate::parser::Token;
use std::convert::TryFrom;
use std::iter::Peekable;

///ITU-T X.680 | ISO/IEC 8824-1, chapter 8
///
/// # Ordering
/// According to ITU-T X.680 | ISO/IEC 8824-1, 8.6, the canonical order is
/// a) Universal, Application, ContextSpecific and Private and
/// b) within each class, the numbers shall be ordered ascending
///
/// ```rust
/// use asn1rs_model::model::Tag;
/// let mut tags = vec![
///     Tag::Universal(1),
///     Tag::Application(0),
///     Tag::Private(7),
///     Tag::ContextSpecific(107),
///     Tag::ContextSpecific(32),
///     Tag::Universal(0),
/// ];
/// tags.sort();
/// assert_eq!(tags, vec![
///     Tag::Universal(0),
///     Tag::Universal(1),
///     Tag::Application(0),
///     Tag::ContextSpecific(32),
///     Tag::ContextSpecific(107),
///     Tag::Private(7),
/// ]);
/// ```
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum Tag {
    Universal(usize),
    Application(usize),
    ContextSpecific(usize),
    Private(usize),
}

impl Tag {
    pub const DEFAULT_BOOLEAN: Tag = Tag::Universal(1);
    pub const DEFAULT_INTEGER: Tag = Tag::Universal(2);
    pub const DEFAULT_BIT_STRING: Tag = Tag::Universal(3);
    pub const DEFAULT_OCTET_STRING: Tag = Tag::Universal(4);
    pub const DEFAULT_NULL: Tag = Tag::Universal(5);
    pub const DEFAULT_ENUMERATED: Tag = Tag::Universal(10);
    pub const DEFAULT_UTF8_STRING: Tag = Tag::Universal(12);
    pub const DEFAULT_SEQUENCE: Tag = Tag::Universal(16);
    pub const DEFAULT_SEQUENCE_OF: Tag = Tag::Universal(16);
    pub const DEFAULT_SET: Tag = Tag::Universal(17);
    pub const DEFAULT_SET_OF: Tag = Tag::Universal(17);

    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_NUMERIC_STRING: Tag = Tag::Universal(18);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_PRINTABLE_STRING: Tag = Tag::Universal(19);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_TELETEXT_STRING: Tag = Tag::Universal(20);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_VIDEOTEXT_STRING: Tag = Tag::Universal(21);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_IA5_STRING: Tag = Tag::Universal(22);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_GRAPHIC_STRING: Tag = Tag::Universal(25);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_VISIBLE_STRING: Tag = Tag::Universal(26);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_GENERAL_STRING: Tag = Tag::Universal(27);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_UNIVERSAL_STRING: Tag = Tag::Universal(28);
    /// ITU-T Rec. X.680, 41
    pub const DEFAULT_BMP_STRING: Tag = Tag::Universal(30);
}

impl<T: Iterator<Item = Token>> TryFrom<&mut Peekable<T>> for Tag {
    type Error = Error;

    fn try_from(iter: &mut Peekable<T>) -> Result<Self, Self::Error> {
        macro_rules! parse_tag_number {
            () => {
                parse_tag_number!(iter.next_or_err()?)
            };
            ($tag:expr) => {{
                let tag = $tag;
                tag.text()
                    .and_then(|t| t.parse().ok())
                    .ok_or_else(|| Error::invalid_tag(tag))?
            }};
        }

        Ok(match iter.next_or_err()? {
            t if t.eq_text_ignore_ascii_case("UNIVERSAL") => Tag::Universal(parse_tag_number!()),
            t if t.eq_text_ignore_ascii_case("APPLICATION") => {
                Tag::Application(parse_tag_number!())
            }
            t if t.eq_text_ignore_ascii_case("PRIVATE") => Tag::Private(parse_tag_number!()),
            t if t.text().is_some() => Tag::ContextSpecific(parse_tag_number!(t)),
            t => return Err(Error::no_text(t)),
        })
    }
}

pub trait TagProperty {
    fn tag(&self) -> Option<Tag>;

    fn set_tag(&mut self, tag: Tag);

    fn reset_tag(&mut self);

    fn with_tag_opt(self, tag: Option<Tag>) -> Self
    where
        Self: Sized,
    {
        if let Some(tag) = tag {
            self.with_tag(tag)
        } else {
            self.without_tag()
        }
    }

    fn with_tag(mut self, tag: Tag) -> Self
    where
        Self: Sized,
    {
        self.set_tag(tag);
        self
    }

    fn without_tag(mut self) -> Self
    where
        Self: Sized,
    {
        self.reset_tag();
        self
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn test_property<T: TagProperty>(mut property: T)
    where
        T: Sized,
    {
        property.set_tag(Tag::Universal(22));
        assert_eq!(Some(Tag::Universal(22)), property.tag());

        property.reset_tag();
        assert_eq!(None, property.tag());

        let property = property.with_tag_opt(Some(Tag::Application(1337)));
        assert_eq!(Some(Tag::Application(1337)), property.tag());

        let property = property.without_tag();
        assert_eq!(None, property.tag());

        let property = property.with_tag(Tag::Private(42));
        assert_eq!(Some(Tag::Private(42)), property.tag());
    }
}
