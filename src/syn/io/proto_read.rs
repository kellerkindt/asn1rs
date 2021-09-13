use crate::io::protobuf::ProtoRead as _;
use crate::io::protobuf::{Error, Format};
use crate::syn::*;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::ops::Range;

#[derive(Debug, Clone)]
enum State {
    Root {
        range: Range<usize>,
    },
    Enclosed {
        tag_counter: u32,
        tags: VecDeque<(u32, Format, Range<usize>)>,
    },
}

pub struct ProtobufReader<'a> {
    source: Cow<'a, [u8]>,
    state: State,
}

impl<'a> From<&'a [u8]> for ProtobufReader<'a> {
    fn from(slice: &'a [u8]) -> Self {
        Self {
            state: State::Root {
                range: 0..slice.len(),
            },
            source: Cow::Borrowed(slice),
        }
    }
}

impl From<Vec<u8>> for ProtobufReader<'static> {
    fn from(vec: Vec<u8>) -> Self {
        Self {
            state: State::Root {
                range: 0..vec.len(),
            },
            source: Cow::Owned(vec),
        }
    }
}

impl<'a> ProtobufReader<'a> {
    fn index_enclosed(&self, range: Range<usize>) -> Result<State, <Self as Reader>::Error> {
        let mut position = range.start;
        let mut tags = VecDeque::new();

        while position < range.end {
            let slice = &self.source[position..range.end];
            let pos_before = slice.len();
            let reader = &mut &*slice;
            let (tag, format) = reader.read_tag()?;
            let pos_after = reader.len();
            let content_position = position + (pos_before - pos_after);
            let (content_offset, content_length) =
                Self::read_content_offset_and_length(reader, format)?;
            let content_position = content_position + content_offset;
            let content_end = content_position + content_length;

            tags.push_back((tag, format, content_position..content_end));
            eprintln!("{:?}", tags);
            position = content_end;
        }

        eprintln!("tags {:?}", tags);

        Ok(State::Enclosed {
            tag_counter: 1,
            tags,
        })
    }

    fn read_content_offset_and_length(
        slice: &mut &[u8],
        format: Format,
    ) -> Result<(usize, usize), <Self as Reader>::Error> {
        match format {
            Format::VarInt => {
                let len_before = slice.len();
                slice.read_varint()?;
                let len_after = slice.len();
                Ok((0, len_before - len_after))
            }
            Format::Fixed64 => Ok((0, 8)),
            Format::LengthDelimited => {
                let len_before = slice.len();
                let content_length = slice.read_varint()?;
                let len_after = slice.len();
                Ok((len_before - len_after, content_length as usize))
            }
            Format::Fixed32 => Ok((0, 4)),
        }
    }

    fn hast_next_tag(&self) -> bool {
        match &self.state {
            State::Root { .. } => true,
            State::Enclosed {
                tag_counter, tags, ..
            } => {
                let next_tag = *tag_counter;
                tags.iter().any(|(tag, _format, _range)| *tag == next_tag)
            }
        }
    }

    fn increment_tag_counter(&mut self) {
        match &mut self.state {
            State::Root { .. } => {}
            State::Enclosed { tag_counter, .. } => {
                *tag_counter += 1;
            }
        }
    }

    fn next_tag_range<const INCREMENT: bool>(&mut self) -> Option<Range<usize>> {
        self.next_tag_range_format_opt::<INCREMENT>(None)
    }

    fn next_tag_range_filter_format<const INCREMENT: bool>(
        &mut self,
        format: Format,
    ) -> Option<Range<usize>> {
        self.next_tag_range_format_opt::<INCREMENT>(Some(format))
    }

    #[inline]
    fn next_tag_range_format_opt<const INCREMENT: bool>(
        &mut self,
        format_filter: Option<Format>,
    ) -> Option<Range<usize>> {
        match &mut self.state {
            State::Root { range } => Some(range.clone()),
            State::Enclosed { tag_counter, tags } => {
                let next_tag = *tag_counter;

                if INCREMENT {
                    *tag_counter += 1;
                }

                let index_range_format =
                    tags.iter()
                        .enumerate()
                        .find_map(|(index, (tag, format, range))| {
                            if *tag == next_tag && format_filter.map_or(true, |f| f == *format) {
                                Some((index, range.clone()))
                            } else {
                                None
                            }
                        });

                match index_range_format {
                    Some((index, range)) => {
                        tags.remove(index);
                        Some(range)
                    }
                    None => None,
                }
            }
        }
    }

    fn next_range_format_reader(&mut self, format: Format) -> &[u8] {
        let range = self
            .next_tag_range_filter_format::<true>(format)
            .unwrap_or(0..0);
        &self.source[range]
    }

    #[inline]
    fn read_set_or_sequence<S: Sized, F: Fn(&mut Self) -> Result<S, <Self as Reader>::Error>>(
        &mut self,
        f: F,
    ) -> Result<S, <Self as Reader>::Error> {
        let range = self
            .next_tag_range_filter_format::<true>(Format::LengthDelimited)
            .unwrap_or(0..0);

        let mut state = self.index_enclosed(range)?;

        core::mem::swap(&mut self.state, &mut state);
        let result = f(self);
        self.state = state;

        result
    }

    #[inline]
    fn read_set_or_sequence_of<T: ReadableType>(
        &mut self,
    ) -> Result<Vec<<T as ReadableType>::Type>, <Self as Reader>::Error> {
        let mut vec = Vec::new();

        while let Some(range) = self.next_tag_range::<false>() {
            let mut state = State::Root { range };
            core::mem::swap(&mut self.state, &mut state);
            vec.push(T::read_value(self)?);
            self.state = state;
        }

        self.increment_tag_counter();
        Ok(vec)
    }
}

impl<'a> Reader for ProtobufReader<'a> {
    type Error = Error;

    #[inline]
    fn read_sequence<
        C: sequence::Constraint,
        S: Sized,
        F: Fn(&mut Self) -> Result<S, Self::Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<S, Self::Error> {
        self.read_set_or_sequence(f)
    }

    #[inline]
    fn read_sequence_of<C: sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<<T as ReadableType>::Type>, Self::Error> {
        self.read_set_or_sequence_of::<T>()
    }

    #[inline]
    fn read_set<C: set::Constraint, S: Sized, F: Fn(&mut Self) -> Result<S, Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<S, Self::Error> {
        self.read_set_or_sequence(f)
    }

    #[inline]
    fn read_set_of<C: setof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<<T as ReadableType>::Type>, Self::Error> {
        self.read_set_or_sequence_of::<T>()
    }

    #[inline]
    fn read_enumerated<C: enumerated::Constraint>(&mut self) -> Result<C, Self::Error> {
        let index = if let Some(range) = self.next_tag_range_filter_format::<true>(Format::VarInt) {
            let reader = &mut &self.source[range];
            reader.read_varint()?
        } else {
            0
        };

        C::from_choice_index(index).ok_or_else(|| Error::invalid_variant(index))
    }

    #[inline]
    fn read_choice<C: choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        match self.next_tag_range::<true>() {
            None => Err(Error::MissingRequiredField(C::NAME)),
            Some(range) => {
                eprintln!("{}: range={:?}", C::NAME, range);

                let (format, range, tag) = {
                    let reader = &mut &self.source[range.clone()];
                    let len_before = reader.len();
                    let (tag, format) = reader.read_tag()?;
                    if format == Format::LengthDelimited {
                        let _len = reader.read_varint()?;
                    }
                    let len_after = reader.len();
                    let read = len_before - len_after;
                    (format, range.start + read..range.end, tag)
                };

                let mut state = State::Enclosed {
                    tag_counter: 1,
                    tags: {
                        let mut v = VecDeque::with_capacity(1);
                        v.push_back((1u32, format, range));
                        v
                    },
                };
                core::mem::swap(&mut self.state, &mut state);
                let result = C::read_content(u64::from(tag.saturating_sub(1)), self);
                self.state = state;

                match result {
                    Err(e) => Err(e),
                    Ok(None) => Err(Error::unexpected_tag((tag, Format::LengthDelimited))),
                    Ok(Some(v)) => Ok(v),
                }
            }
        }
    }

    #[inline]
    fn read_opt<T: ReadableType>(
        &mut self,
    ) -> Result<Option<<T as ReadableType>::Type>, Self::Error> {
        if self.hast_next_tag() {
            T::read_value(self).map(Some)
        } else {
            self.increment_tag_counter();
            Ok(None)
        }
    }

    #[inline]
    fn read_default<C: default::Constraint<Owned = T::Type>, T: ReadableType>(
        &mut self,
    ) -> Result<T::Type, Self::Error> {
        // todo is there a better solution than to ignore this?
        T::read_value(self)
    }

    #[inline]
    fn read_number<T: numbers::Number, C: numbers::Constraint<T>>(
        &mut self,
    ) -> Result<T, Self::Error> {
        let mut reader = self.next_range_format_reader(Format::VarInt);

        // protobuf does not serialize null or 0-ish values
        if reader.is_empty() {
            return Ok(T::from_i64(0));
        }

        // This way is clearer, that the first branch is for unsigned and the second branch for
        // signed types, while the inner branches determine 32- or 64-bitness
        #[allow(clippy::collapsible_if)]
        if const_unwrap_or!(C::MIN, 0) >= 0 {
            if const_unwrap_or!(C::MAX, i64::MAX) <= i64::from(u32::MAX) {
                reader.read_uint32().map(|v| T::from_i64(v as i64))
            } else {
                reader.read_uint64().map(|v| T::from_i64(v as i64))
            }
        } else if const_unwrap_or!(C::MIN, i64::MIN) >= i64::from(i32::MIN)
            && const_unwrap_or!(C::MAX, i64::MAX) <= i64::from(i32::MAX)
        {
            reader.read_sint32().map(|v| T::from_i64(v as i64))
        } else {
            reader.read_sint64().map(|v| T::from_i64(v as i64))
        }
    }

    #[inline]
    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let mut reader = self.next_range_format_reader(Format::LengthDelimited);
        reader.read_string()
    }

    #[inline]
    fn read_ia5string<C: ia5string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let mut reader = self.next_range_format_reader(Format::LengthDelimited);
        reader.read_string()
    }

    #[inline]
    fn read_numeric_string<C: numericstring::Constraint>(&mut self) -> Result<String, Self::Error> {
        let mut reader = self.next_range_format_reader(Format::LengthDelimited);
        reader.read_string()
    }

    #[inline]
    fn read_printable_string<C: printablestring::Constraint>(
        &mut self,
    ) -> Result<String, Self::Error> {
        let mut reader = self.next_range_format_reader(Format::LengthDelimited);
        reader.read_string()
    }

    #[inline]
    fn read_visible_string<C: visiblestring::Constraint>(&mut self) -> Result<String, Self::Error> {
        let mut reader = self.next_range_format_reader(Format::LengthDelimited);
        reader.read_string()
    }

    #[inline]
    fn read_octet_string<C: octetstring::Constraint>(&mut self) -> Result<Vec<u8>, Self::Error> {
        let mut reader = self.next_range_format_reader(Format::LengthDelimited); // TODO Format::VarInt ??
        reader.read_bytes()
    }

    #[inline]
    fn read_bit_string<C: bitstring::Constraint>(&mut self) -> Result<(Vec<u8>, u64), Self::Error> {
        let mut reader = self.next_range_format_reader(Format::LengthDelimited); // TODO Format::VarInt ??
        let bytes = reader.read_bytes()?;
        let bits = BitVec::from_vec_with_trailing_bit_len(bytes);
        Ok(bits.split())
    }

    #[inline]
    fn read_boolean<C: boolean::Constraint>(&mut self) -> Result<bool, Self::Error> {
        let mut reader = self.next_range_format_reader(Format::VarInt);

        // protobuf does not serialize null or 0-ish values
        if reader.is_empty() {
            return Ok(false);
        }

        reader.read_bool()
    }

    #[inline]
    fn read_null<C: null::Constraint>(&mut self) -> Result<Null, Self::Error> {
        Ok(Null)
    }
}
