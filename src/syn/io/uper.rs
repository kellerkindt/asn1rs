use crate::io::buffer::BitBuffer;
use crate::io::uper::Error as UperError;
use crate::io::uper::Reader as _UperReader;
use crate::io::uper::Writer as _UperWriter;
use crate::prelude::*;
use std::ops::Range;

#[derive(Default)]
pub struct UperWriter {
    buffer: BitBuffer,
    scope: Option<Range<usize>>,
}

impl UperWriter {
    pub fn byte_content(&self) -> &[u8] {
        self.buffer.content()
    }

    pub const fn bit_len(&self) -> usize {
        self.buffer.bit_len()
    }

    pub fn into_bytes_vec(self) -> Vec<u8> {
        self.buffer.into()
    }

    pub fn into_reader(self) -> UperReader {
        let bits = self.bit_len();
        let bytes = self.into_bytes_vec();
        UperReader::from_bits(bytes, bits)
    }

    #[inline]
    pub fn scope_pushed<R, F: Fn(&mut Self) -> R>(&mut self, scope: Range<usize>, f: F) -> R {
        let original = core::mem::replace(&mut self.scope, Some(scope));
        let result = f(self);
        let scope = core::mem::replace(&mut self.scope, original);
        let scope = scope.unwrap(); // save because this is the original from above
        debug_assert_eq!(scope.start, scope.end);
        result
    }

    #[inline]
    pub fn scope_stashed<R, F: Fn(&mut Self) -> R>(&mut self, f: F) -> R {
        let scope = self.scope.take();
        let result = f(self);
        self.scope = scope;
        result
    }
}

impl Writer for UperWriter {
    type Error = UperError;

    #[inline]
    fn write_sequence<C: sequence::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error> {
        // In UPER the values for all OPTIONAL flags are written before any field
        // value is written. This remembers their position, so a later call of `write_opt`
        // can write them to the buffer
        let write_pos = self.buffer.write_position;
        let range = write_pos..write_pos + C::OPTIONAL_FIELDS; // TODO
        for _ in 0..C::OPTIONAL_FIELDS {
            // insert in reverse order so that a simple pop() in `write_opt` retrieves
            // the relevant position
            if let Err(e) = self.buffer.write_bit(false) {
                self.buffer.write_position = write_pos; // undo write_bits
                return Err(e);
            }
        }

        self.scope_pushed(range, f)
    }

    #[inline]
    fn write_sequence_of<C: sequenceof::Constraint, T: WritableType>(
        &mut self,
        slice: &[T::Type],
    ) -> Result<(), Self::Error> {
        let min = C::MIN.unwrap_or(0);
        let max = C::MAX.unwrap_or(std::usize::MAX);
        if slice.len() < min || slice.len() > max {
            return Err(UperError::SizeNotInRange(slice.len(), min, max));
        }
        self.scope_stashed(|w| {
            w.buffer.write_length_determinant(slice.len() - min)?; // TODO untested for MIN != 0
            for value in slice {
                T::write_value(w, value)?;
            }
            Ok(())
        })
    }

    #[inline]
    fn write_enumerated<C: enumerated::Constraint>(
        &mut self,
        enumerated: &C,
    ) -> Result<(), Self::Error> {
        if C::EXTENSIBLE {
            self.buffer.write_choice_index_extensible(
                enumerated.to_choice_index() as u64,
                C::STD_VARIANT_COUNT as u64,
            )
        } else {
            self.buffer.write_choice_index(
                enumerated.to_choice_index() as u64,
                C::STD_VARIANT_COUNT as u64,
            )
        }
    }

    #[inline]
    fn write_choice<C: choice::Constraint>(&mut self, choice: &C) -> Result<(), Self::Error> {
        self.scope_stashed(|w| {
            if C::EXTENSIBLE {
                let index = choice.to_choice_index();
                w.buffer
                    .write_choice_index_extensible(index as u64, C::STD_VARIANT_COUNT as u64)?;
                if index >= C::STD_VARIANT_COUNT {
                    // TODO performance
                    let mut writer = UperWriter::default();
                    choice.write_content(&mut writer)?;
                    w.buffer
                        .write_length_determinant(writer.byte_content().len())?;
                    return w
                        .buffer
                        .write_bit_string_till_end(&writer.byte_content(), 0);
                }
            } else {
                w.buffer.write_choice_index(
                    choice.to_choice_index() as u64,
                    C::STD_VARIANT_COUNT as u64,
                )?;
            }
            choice.write_content(w)
        })
    }

    #[inline]
    fn write_opt<T: WritableType>(
        &mut self,
        value: Option<&<T as WritableType>::Type>,
    ) -> Result<(), Self::Error> {
        if let Some(range) = &mut self.scope {
            if range.start < range.end {
                let result = self
                    .buffer
                    .with_write_position_at(range.start, |b| b.write_bit(value.is_some()));
                range.start += 1;
                result?;
            } else {
                return Err(UperError::OptFlagsExhausted);
            }
        } else {
            self.buffer.write_bit(value.is_some())?;
        }
        if let Some(value) = value {
            self.scope_stashed(|w| T::write_value(w, value))
        } else {
            Ok(())
        }
    }

    #[inline]
    fn write_int(&mut self, value: i64, range: (i64, i64)) -> Result<(), Self::Error> {
        self.buffer.write_int(value, range)
    }

    #[inline]
    fn write_int_max(&mut self, value: u64) -> Result<(), Self::Error> {
        self.buffer.write_int_max(value)
    }

    #[inline]
    fn write_utf8string<C: utf8string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        self.buffer.write_utf8_string(value)
    }

    #[inline]
    fn write_octet_string<C: octetstring::Constraint>(
        &mut self,
        value: &[u8],
    ) -> Result<(), Self::Error> {
        self.buffer
            .write_octet_string(value, bit_buffer_range::<C>())
    }

    #[inline]
    fn write_boolean<C: boolean::Constraint>(&mut self, value: bool) -> Result<(), Self::Error> {
        self.buffer.write_bit(value)
    }
}

pub struct UperReader {
    buffer: BitBuffer,
    scope: Option<Range<usize>>,
}

impl UperReader {
    pub fn from_bits<I: Into<Vec<u8>>>(bytes: I, bit_len: usize) -> Self {
        Self {
            buffer: BitBuffer::from_bits(bytes.into(), bit_len),
            scope: Default::default(),
        }
    }

    #[inline]
    pub const fn bits_remaining(&self) -> usize {
        self.buffer.write_position - self.buffer.read_position
    }

    #[inline]
    pub fn scope_pushed<R, F: Fn(&mut Self) -> R>(&mut self, scope: Range<usize>, f: F) -> R {
        let original = core::mem::replace(&mut self.scope, Some(scope));
        let result = f(self);
        let scope = core::mem::replace(&mut self.scope, original);
        debug_assert_eq!(scope.clone().unwrap().start, scope.unwrap().end); // save because this is the original from above
        result
    }

    #[inline]
    pub fn scope_stashed<R, F: Fn(&mut Self) -> R>(&mut self, f: F) -> R {
        let scope = self.scope.take();
        let result = f(self);
        self.scope = scope;
        result
    }

    #[inline]
    pub fn read_whole_sub_slice<T, E, F: Fn(&mut Self) -> Result<T, E>>(
        &mut self,
        length_bytes: usize,
        f: F,
    ) -> Result<T, E> {
        let write_position = self.buffer.read_position + (length_bytes * 8);
        let write_original = core::mem::replace(&mut self.buffer.write_position, write_position);
        let result = f(self);
        // extend to original position
        self.buffer.write_position = write_original;
        if result.is_ok() {
            // on successful read, skip the slice
            self.buffer.read_position = write_position;
        }
        result
    }
}

impl Reader for UperReader {
    type Error = UperError;

    #[inline]
    fn read_sequence<
        C: sequence::Constraint,
        S: Sized,
        F: Fn(&mut Self) -> Result<S, Self::Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<S, Self::Error> {
        // In UPER the values for all OPTIONAL flags are written before any field
        // value is written. This remembers their position, so a later call of `read_opt`
        // can retrieve them from the buffer
        let range = self.buffer.read_position..self.buffer.read_position + C::OPTIONAL_FIELDS;
        if self.buffer.bit_len() < range.end {
            return Err(UperError::EndOfStream);
        }
        self.buffer.read_position = range.end; // skip optional
        self.scope_pushed(range, f)
    }

    #[inline]
    fn read_sequence_of<C: sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<T::Type>, Self::Error> {
        let min = C::MIN.unwrap_or(0);
        let max = C::MAX.unwrap_or(std::usize::MAX);
        let len = self.buffer.read_length_determinant()? + min; // TODO untested for MIN != 0
        if len > max {
            Err(UperError::SizeNotInRange(len, min, max))
        } else {
            self.scope_stashed(|w| {
                let mut vec = Vec::with_capacity(len);
                for _ in 0..len {
                    vec.push(T::read_value(w)?);
                }
                Ok(vec)
            })
        }
    }

    #[inline]
    fn read_enumerated<C: enumerated::Constraint>(&mut self) -> Result<C, Self::Error> {
        if C::EXTENSIBLE {
            self.buffer
                .read_choice_index_extensible(C::STD_VARIANT_COUNT as u64)
                .map(|v| v as usize)
        } else {
            self.buffer
                .read_choice_index(C::STD_VARIANT_COUNT as u64)
                .map(|v| v as usize)
        }
        .and_then(|index| {
            C::from_choice_index(index)
                .ok_or_else(|| UperError::InvalidChoiceIndex(index, C::VARIANT_COUNT))
        })
    }

    #[inline]
    fn read_choice<C: choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        self.scope_stashed(|r| {
            if C::EXTENSIBLE {
                let index = r
                    .buffer
                    .read_choice_index_extensible(C::STD_VARIANT_COUNT as u64)
                    .map(|v| v as usize)?;
                if index >= C::STD_VARIANT_COUNT {
                    let byte_len = r.buffer.read_length_determinant()?;
                    r.read_whole_sub_slice(byte_len, |r| Ok((index, C::read_content(index, r)?)))
                } else {
                    Ok((index, C::read_content(index, r)?))
                }
            } else {
                r.buffer
                    .read_choice_index(C::STD_VARIANT_COUNT as u64)
                    .map(|v| v as usize)
                    .and_then(|index| Ok((index, C::read_content(index, r)?)))
            }
            .and_then(|(index, content)| {
                content.ok_or_else(|| UperError::InvalidChoiceIndex(index, C::VARIANT_COUNT))
            })
        })
    }

    #[inline]
    fn read_opt<T: ReadableType>(
        &mut self,
    ) -> Result<Option<<T as ReadableType>::Type>, Self::Error> {
        let value = if let Some(range) = &mut self.scope {
            if range.start < range.end {
                let result = self
                    .buffer
                    .with_read_position_at(range.start, |b| b.read_bit());
                range.start += 1;
                result?
            } else {
                return Err(UperError::OptFlagsExhausted);
            }
        } else {
            self.buffer.read_bit()?
        };
        if value {
            self.scope_stashed(T::read_value).map(Some)
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn read_int(&mut self, range: (i64, i64)) -> Result<i64, Self::Error> {
        self.buffer.read_int(range)
    }

    #[inline]
    fn read_int_max(&mut self) -> Result<u64, Self::Error> {
        self.buffer.read_int_max()
    }

    #[inline]
    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error> {
        self.buffer.read_utf8_string()
    }

    #[inline]
    fn read_octet_string<C: octetstring::Constraint>(&mut self) -> Result<Vec<u8>, Self::Error> {
        self.buffer.read_octet_string(bit_buffer_range::<C>())
    }

    #[inline]
    fn read_boolean<C: boolean::Constraint>(&mut self) -> Result<bool, Self::Error> {
        self.buffer.read_bit()
    }
}

#[inline]
fn bit_buffer_range<C: octetstring::Constraint>() -> Option<(i64, i64)> {
    match (C::MIN, C::MAX) {
        (None, None) => None,
        (min, max) => Some((
            min.unwrap_or(0) as i64,
            max.unwrap_or(std::i64::MAX as usize) as i64, // TODO never verified!
        )),
    }
}
