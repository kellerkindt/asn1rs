use crate::io::protobuf::ProtoRead as _;
use crate::io::protobuf::{Error, Format};
use crate::syn::*;

#[derive(Clone, Copy)]
struct State<'a> {
    source: &'a [u8],
    tag_counter: u32,
}

pub struct ProtobufReader<'a> {
    state: State<'a>,
    is_root: bool,
}

impl<'a> From<&'a [u8]> for ProtobufReader<'a> {
    fn from(slice: &'a [u8]) -> Self {
        Self {
            state: State {
                source: slice,
                tag_counter: 0,
            },
            is_root: true,
        }
    }
}

impl<'a> ProtobufReader<'a> {
    #[inline]
    fn read_tag_format(&mut self, tag: u32, format: Format) -> Result<(), Error> {
        let format_read = self.read_tag(tag)?;
        if format_read != format {
            Err(Error::unexpected_format(format_read))
        } else {
            Ok(())
        }
    }

    #[inline]
    fn read_tag(&mut self, tag: u32) -> Result<Format, Error> {
        let (read_tag, format) = self.state.source.read_tag()?;

        if tag == read_tag {
            Ok(format)
        } else {
            Err(Error::invalid_tag_received(tag))
        }
    }

    #[inline]
    fn read_set_or_sequence<S: Sized, F: Fn(&mut Self) -> Result<S, <Self as Reader>::Error>>(
        &mut self,
        f: F,
    ) -> Result<S, <Self as Reader>::Error> {
        let root = core::mem::take(&mut self.is_root);
        let state = self.state;

        let result = if root {
            f(self)
        } else {
            let tag = self.state.tag_counter + 1;
            let format = self.read_tag(tag)?;

            if Format::LengthDelimited == format {
                let len = self.state.source.read_varint()?;
                let (content, remaining) = self.state.source.split_at(len as usize);

                self.state.tag_counter = 0;
                self.state.source = content;

                f(self).map(|v| {
                    self.state.source = remaining;
                    v
                })
            } else {
                f(self)
            }
        };

        self.state.tag_counter = state.tag_counter + 1;
        self.is_root = root;
        result
    }

    #[inline]
    fn read_set_or_sequence_of<T: ReadableType>(
        &mut self,
    ) -> Result<Vec<<T as ReadableType>::Type>, <Self as Reader>::Error> {
        let mut vec = Vec::default();
        let tag = self.state.tag_counter + 1;

        while !self.state.source.is_empty() {
            let mut prober = &self.state.source[..];
            let (probed_tag, _format) = prober.read_tag()?;
            if probed_tag == tag {
                self.state.tag_counter = tag - 1;
                vec.push(T::read_value(self)?);
            } else {
                break;
            }
        }

        self.state.tag_counter = tag;
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
        if !self.is_root {
            self.read_tag_format(self.state.tag_counter + 1, Format::VarInt)?;
        }
        self.state.tag_counter += 1;
        let index = self.state.source.read_varint()?;
        C::from_choice_index(index).ok_or_else(|| Error::invalid_variant(index))
    }

    #[inline]
    fn read_choice<C: choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        let root = core::mem::take(&mut self.is_root);
        let state = self.state;

        let content_reader = |this: &mut Self| {
            let mut reader = &this.state.source[..];
            let (tag, format) = reader.read_tag()?;
            this.state.tag_counter = tag.saturating_sub(1);
            match C::read_content(u64::from(this.state.tag_counter), this) {
                Err(e) => Err(e),
                Ok(None) => Err(Error::unexpected_tag((tag, format))),
                Ok(Some(v)) => Ok(v),
            }
        };

        let result = if root {
            content_reader(self)
        } else {
            let tag = self.state.tag_counter + 1;
            let format = self.read_tag(tag)?;

            if Format::LengthDelimited == format {
                let len = self.state.source.read_varint()?;
                let (content, remaining) = self.state.source.split_at(len as usize);

                self.state.source = content;
                content_reader(self).map(|v| {
                    self.state.source = remaining;
                    v
                })
            } else {
                content_reader(self)
            }
        };

        self.state.tag_counter = state.tag_counter + 1;
        self.is_root = root;
        result
    }

    #[inline]
    fn read_opt<T: ReadableType>(
        &mut self,
    ) -> Result<Option<<T as ReadableType>::Type>, Self::Error> {
        if self.state.source.is_empty() {
            self.state.tag_counter += 1;
            Ok(None)
        } else {
            let mut reader = &self.state.source[..];
            let tag = reader.read_tag()?.0;

            if tag == self.state.tag_counter + 1 {
                T::read_value(self).map(Some)
            } else {
                self.state.tag_counter += 1;
                Ok(None)
            }
        }
    }

    #[inline]
    fn read_number<T: numbers::Number, C: numbers::Constraint<T>>(
        &mut self,
    ) -> Result<T, Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.read_tag_format(tag, Format::VarInt)?;
        self.state.tag_counter = tag;

        // This way is clearer, that the first branch is for unsigned and the second branch for
        // signed types, while the inner branches determine 32- or 64-bitness
        #[allow(clippy::collapsible_if)]
        if const_unwrap_or!(C::MIN, 0) >= 0 {
            if const_unwrap_or!(C::MAX, i64::MAX) <= i64::from(u32::MAX) {
                self.state
                    .source
                    .read_uint32()
                    .map(|v| T::from_i64(v as i64))
            } else {
                self.state
                    .source
                    .read_uint64()
                    .map(|v| T::from_i64(v as i64))
            }
        } else {
            if const_unwrap_or!(C::MIN, i64::MIN) >= i64::from(i32::MIN)
                && const_unwrap_or!(C::MAX, i64::MAX) <= i64::from(i32::MAX)
            {
                self.state
                    .source
                    .read_sint32()
                    .map(|v| T::from_i64(v as i64))
            } else {
                self.state
                    .source
                    .read_sint64()
                    .map(|v| T::from_i64(v as i64))
            }
        }
    }

    #[inline]
    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.read_tag_format(tag, Format::LengthDelimited)?;
        let string = self.state.source.read_string()?;
        self.state.tag_counter = tag;
        Ok(string)
    }

    #[inline]
    fn read_ia5string<C: ia5string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.read_tag_format(tag, Format::LengthDelimited)?;
        let string = self.state.source.read_string()?;
        self.state.tag_counter = tag;
        Ok(string)
    }

    #[inline]
    fn read_numeric_string<C: numericstring::Constraint>(&mut self) -> Result<String, Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.read_tag_format(tag, Format::LengthDelimited)?;
        let string = self.state.source.read_string()?;
        self.state.tag_counter = tag;
        Ok(string)
    }

    #[inline]
    fn read_printable_string<C: printablestring::Constraint>(
        &mut self,
    ) -> Result<String, Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.read_tag_format(tag, Format::LengthDelimited)?;
        let string = self.state.source.read_string()?;
        self.state.tag_counter = tag;
        Ok(string)
    }

    #[inline]
    fn read_octet_string<C: octetstring::Constraint>(&mut self) -> Result<Vec<u8>, Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.read_tag_format(tag, Format::VarInt)?;
        let bytes = self.state.source.read_bytes()?;
        self.state.tag_counter = tag;
        Ok(bytes)
    }

    #[inline]
    fn read_bit_string<C: bitstring::Constraint>(&mut self) -> Result<(Vec<u8>, u64), Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.read_tag_format(tag, Format::VarInt)?;
        let bytes = self.state.source.read_bytes()?;
        let bits = BitVec::from_vec_with_trailing_bit_len(bytes);
        self.state.tag_counter = tag;
        Ok(bits.split())
    }

    #[inline]
    fn read_boolean<C: boolean::Constraint>(&mut self) -> Result<bool, Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.read_tag_format(tag, Format::VarInt)?;
        self.state.tag_counter = tag;
        self.state.source.read_bool()
    }
}
