use crate::io::protobuf::ProtoWrite as _;
use crate::io::protobuf::{Error, Format};
use crate::prelude::ProtobufReader;
use crate::syn::*;
use std::io::Write;

#[derive(Debug, Default, Copy, Clone)]
struct State {
    tag_counter: u32,
    format: Option<Format>,
}

enum SliceOrVec<'a> {
    Vec(Vec<u8>),
    Slice(usize, &'a mut [u8]),
}

impl SliceOrVec<'_> {
    pub fn into_inner_vec(self) -> Option<Vec<u8>> {
        match self {
            Self::Vec(vec) => Some(vec),
            Self::Slice(_, _) => None,
        }
    }
}

impl std::io::Write for SliceOrVec<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Vec(vec) => vec.write(buf),
            Self::Slice(written, slice) => {
                let (_left, mut right) = slice.split_at_mut(*written);
                let len_before = right.len();
                let result = right.write(buf);
                let len_after = right.len();
                *written += len_before.saturating_sub(len_after);
                result
            }
        }
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        // this is a no-op because there is no buffer in-front of the buffer
        match self {
            Self::Vec(..) | Self::Slice(..) => Ok(()),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Vec(vec) => vec.write_all(buf),
            Self::Slice(written, slice) => {
                let (_left, mut right) = slice.split_at_mut(*written);
                let len_before = right.len();
                let result = right.write_all(buf);
                let len_after = right.len();
                *written += len_before.saturating_sub(len_after);
                result
            }
        }
    }
}

impl Default for SliceOrVec<'_> {
    fn default() -> Self {
        Self::Vec(Vec::default())
    }
}

pub struct ProtobufWriter<'a> {
    buffer: SliceOrVec<'a>,
    state: State,
    is_root: bool,
}

impl Default for ProtobufWriter<'_> {
    fn default() -> Self {
        Self {
            buffer: SliceOrVec::default(),
            state: State::default(),
            is_root: true,
        }
    }
}

impl<'a> From<&'a mut [u8]> for ProtobufWriter<'a> {
    fn from(slice: &'a mut [u8]) -> Self {
        ProtobufWriter {
            buffer: SliceOrVec::Slice(0, slice),
            state: State::default(),
            is_root: true,
        }
    }
}

impl<'a> ProtobufWriter<'a> {
    pub fn into_bytes_vec(self) -> Vec<u8> {
        match self.buffer {
            SliceOrVec::Vec(vec) => vec,
            SliceOrVec::Slice(written, slice) => slice[..written].to_vec(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match &self.buffer {
            SliceOrVec::Vec(vec) => &vec[..],
            SliceOrVec::Slice(written, slice) => &slice[..*written],
        }
    }

    pub fn as_reader(&self) -> ProtobufReader {
        ProtobufReader::from(self.as_bytes())
    }

    pub fn len_written(&self) -> usize {
        match &self.buffer {
            SliceOrVec::Vec(vec) => vec.len(),
            SliceOrVec::Slice(written, _slice) => *written,
        }
    }

    #[inline]
    fn write_set_or_sequence<F: Fn(&mut Self) -> Result<(), <Self as Writer>::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), <Self as Writer>::Error> {
        let root = core::mem::take(&mut self.is_root);
        let mut state = core::mem::take(&mut self.state);

        let result = if !root {
            let tag = state.tag_counter + 1;
            let mut content = core::mem::take(&mut self.buffer);

            let result = f(self);
            core::mem::swap(&mut content, &mut self.buffer);

            if result.is_ok() {
                let content = content.into_inner_vec().unwrap(); // fine because take creates a vec
                self.buffer.write_tag(tag, Format::LengthDelimited)?;
                self.buffer.write_varint(content.len() as u64)?;
                self.buffer.write_all(&content[..])?;
                state.tag_counter = tag;
            }

            result
        } else {
            let result = f(self);
            self.is_root = true;
            result
        };

        self.state = state;
        self.state.format = Some(Format::LengthDelimited);
        result
    }

    #[inline]
    fn write_set_or_sequence_of<T: WritableType>(
        &mut self,
        slice: &[<T as WritableType>::Type],
    ) -> Result<(), <Self as Writer>::Error> {
        let state = self.state;

        for value in slice {
            let result = T::write_value(self, value);
            self.state = state;
            result?;
        }

        self.state.tag_counter += 1;
        //self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }
}

impl Writer for ProtobufWriter<'_> {
    type Error = Error;

    #[inline]
    fn write_sequence<C: sequence::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error> {
        self.write_set_or_sequence(f)
    }

    #[inline]
    fn write_sequence_of<C: sequenceof::Constraint, T: WritableType>(
        &mut self,
        slice: &[<T as WritableType>::Type],
    ) -> Result<(), Self::Error> {
        self.write_set_or_sequence_of::<T>(slice)
    }

    #[inline]
    fn write_set<C: set::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error> {
        self.write_set_or_sequence(f)
    }

    #[inline]
    fn write_set_of<C: setof::Constraint, T: WritableType>(
        &mut self,
        slice: &[<T as WritableType>::Type],
    ) -> Result<(), Self::Error> {
        self.write_set_or_sequence_of::<T>(slice)
    }

    #[inline]
    fn write_enumerated<C: enumerated::Constraint>(
        &mut self,
        enumerated: &C,
    ) -> Result<(), Self::Error> {
        if self.is_root {
            self.buffer
                .write_enum_variant(enumerated.to_choice_index() as u32)?;
        } else {
            let tag = self.state.tag_counter + 1;
            self.buffer
                .write_tagged_enum_variant(tag, enumerated.to_choice_index() as u32)?;
            self.state.tag_counter = tag;
        }
        self.state.format = Some(Format::VarInt);
        Ok(())
    }

    #[inline]
    fn write_choice<C: choice::Constraint>(&mut self, choice: &C) -> Result<(), Self::Error> {
        let root = core::mem::take(&mut self.is_root);

        let result = if !root {
            let mut state = core::mem::take(&mut self.state);
            let mut buffer = core::mem::take(&mut self.buffer);

            // writing to the new buffer
            self.state.tag_counter = choice.to_choice_index() as u32;
            let result = choice.write_content(self);

            // restore the original self attributes
            core::mem::swap(&mut buffer, &mut self.buffer);
            core::mem::swap(&mut state, &mut self.state);

            if result.is_ok() {
                let buffer = buffer.into_inner_vec().unwrap(); // fine because take creates a vec
                let format = Format::LengthDelimited;
                let tag = self.state.tag_counter + 1;
                self.buffer.write_tag(tag, format)?;
                self.buffer.write_bytes(&buffer[..])?;
                self.state.tag_counter = tag;
                self.state.format = Some(format);
            }

            result
        } else {
            self.state.tag_counter = choice.to_choice_index() as u32;
            choice.write_content(self)
        };

        self.state.format = Some(Format::LengthDelimited);
        result
    }

    #[inline]
    fn write_opt<T: WritableType>(
        &mut self,
        value: Option<&<T as WritableType>::Type>,
    ) -> Result<(), Self::Error> {
        if let Some(value) = value {
            T::write_value(self, value)?;
        } else {
            self.state.tag_counter += 1;
        }
        self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }

    #[inline]
    fn write_default<C: default::Constraint<Owned = T::Type>, T: WritableType>(
        &mut self,
        value: &T::Type,
    ) -> Result<(), Self::Error> {
        // todo is there a better solution than to ignore this?
        T::write_value(self, value)
    }

    #[inline]
    fn write_number<T: numbers::Number, C: numbers::Constraint<T>>(
        &mut self,
        value: T,
    ) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;

        // This way is clearer, that the first branch is for unsigned and the second branch for
        // signed types, while the inner branches determine 32- or 64-bitness
        #[allow(clippy::collapsible_if)]
        if const_unwrap_or!(C::MIN, 0) >= 0 {
            if const_unwrap_or!(C::MAX, i64::MAX) <= i64::from(u32::MAX) {
                let value = value.to_i64() as u32; // safe cast because of check above
                self.buffer.write_tagged_uint32(tag, value)?;
            } else {
                let value = value.to_i64() as u64; // safe cast because of first check
                self.buffer.write_tagged_uint64(tag, value)?;
            }
        } else if const_unwrap_or!(C::MIN, i64::MIN) >= i64::from(i32::MIN)
            && const_unwrap_or!(C::MAX, i64::MAX) <= i64::from(i32::MAX)
        {
            let value = value.to_i64() as i32; // safe cast because of check above
            self.buffer.write_tagged_sint32(tag, value)?;
        } else {
            let value = value.to_i64();
            self.buffer.write_tagged_sint64(tag, value)?;
        }
        self.state.format = Some(Format::VarInt);
        self.state.tag_counter = tag;
        Ok(())
    }

    #[inline]
    fn write_utf8string<C: utf8string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.buffer.write_tagged_string(tag, value)?;
        self.state.tag_counter = tag;
        self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }

    #[inline]
    fn write_ia5string<C: ia5string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.buffer.write_tagged_string(tag, value)?;
        self.state.tag_counter = tag;
        self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }

    #[inline]
    fn write_numeric_string<C: numericstring::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.buffer.write_tagged_string(tag, value)?;
        self.state.tag_counter = tag;
        self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }

    #[inline]
    fn write_printable_string<C: printablestring::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.buffer.write_tagged_string(tag, value)?;
        self.state.tag_counter = tag;
        self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }

    #[inline]
    fn write_visible_string<C: visiblestring::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.buffer.write_tagged_string(tag, value)?;
        self.state.tag_counter = tag;
        self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }

    #[inline]
    fn write_octet_string<C: octetstring::Constraint>(
        &mut self,
        value: &[u8],
    ) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.buffer.write_tagged_bytes(tag, value)?;
        self.state.tag_counter = tag;
        self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }

    #[inline]
    fn write_bit_string<C: bitstring::Constraint>(
        &mut self,
        value: &[u8],
        bit_len: u64,
    ) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;
        let mut value = value[..(bit_len as usize + 7) / 8].to_vec();
        bit_len.to_be_bytes().iter().for_each(|b| value.push(*b));

        self.buffer.write_tagged_bytes(tag, &value)?;
        self.state.tag_counter = tag;
        self.state.format = Some(Format::LengthDelimited);
        Ok(())
    }

    #[inline]
    fn write_boolean<C: boolean::Constraint>(&mut self, value: bool) -> Result<(), Self::Error> {
        let tag = self.state.tag_counter + 1;
        self.buffer.write_tagged_bool(tag, value)?;
        self.state.tag_counter = tag;
        self.state.format = Some(Format::VarInt);
        Ok(())
    }

    #[inline]
    fn write_null<C: null::Constraint>(&mut self, _value: &Null) -> Result<(), Self::Error> {
        Ok(())
    }
}
