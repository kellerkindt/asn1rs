use crate::io::per::err::Error;
use crate::io::per::unaligned::buffer::BitBuffer;
use crate::io::per::unaligned::BitRead;
use crate::io::per::unaligned::BitWrite;
use crate::io::per::PackedRead;
use crate::io::per::PackedWrite;
use crate::prelude::*;
use std::ops::Range;

/// This ist enum is the main reason, the new impl is about ~10% slower (2020-09) than the previous/
/// legacy implementation. This dynamic state tracking at runtime could be avoided by passing all
/// values as const generics on each `read_`*/`write_`* call [RFC 2000]. Maybe getting rid of all
/// `mem::replace` calls would also be sufficient.
///
/// [RFC 2000]: https://github.com/rust-lang/rust/issues/44580    
pub enum Scope {
    OptBitField(Range<usize>),
    AllBitField(Range<usize>),
    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, an extensible struct is built as
    ///  - part1
    ///    - `eo`: flag for whether the struct serializes/has payload with extended fields
    ///    - flags for optional fields (only for the non-extended fields!)
    ///    - fields serialized 'inline' (only for the non-extended fields!)
    ///  - part2
    ///    - `eo`: number of extended fields (as normally-small-int)
    ///    - `eo`: presence-flag for each extended field (only OPTIONAL fields seem to
    ///            influence these flags!?)
    ///    - `eo`: fields serialized as
    ///      - length-determinant
    ///      - sub-buffer with actual content
    ///
    /// `eo` for `extensible only` attributes
    ///
    /// To find the beginning of part2 - and thus to be able to insert the secondary-header - one
    /// needs to keep track of the current field number. Also, the position of where to write
    /// the presence flags to must be updated as well.
    ExtensibleSequence {
        opt_bit_field: Option<Range<usize>>,
        calls_until_ext_bitfield: usize,
        number_of_ext_fields: usize,
    },
}

impl Scope {
    #[inline]
    pub const fn exhausted(&self) -> bool {
        match self {
            Scope::OptBitField(range) => range.start == range.end,
            Scope::AllBitField(range) => range.start == range.end,
            Scope::ExtensibleSequence { .. } => false,
        }
    }

    #[inline]
    pub const fn encode_as_open_type_field(&self) -> bool {
        matches!(self, Scope::AllBitField(_))
    }

    #[inline]
    pub fn write_into_field(
        &mut self,
        buffer: &mut BitBuffer,
        is_opt: bool,
        is_present: bool,
    ) -> Result<(), Error> {
        match self {
            Scope::OptBitField(range) => {
                if is_opt {
                    let result =
                        buffer.with_write_position_at(range.start, |b| b.write_bit(is_present));
                    range.start += 1;
                    result
                } else {
                    Ok(())
                }
            }
            Scope::AllBitField(range) => {
                let result =
                    buffer.with_write_position_at(range.start, |b| b.write_bit(is_present));
                range.start += 1;
                result
            }
            Scope::ExtensibleSequence {
                opt_bit_field,
                calls_until_ext_bitfield,
                number_of_ext_fields,
            } => {
                if *calls_until_ext_bitfield == 0 {
                    // when we reach this point, there is never zero numbers of ext-fields
                    buffer.write_normally_small_non_negative_whole_number(
                        *number_of_ext_fields as u64 - 1,
                    )?;
                    let pos = buffer.write_position;
                    for _ in 0..*number_of_ext_fields {
                        if let Err(e) = buffer.write_bit(true) {
                            buffer.write_position = pos;
                            return Err(e);
                        }
                    }
                    let range = pos..buffer.write_position;
                    // buffer.write_int(range.len() as i64, (1, range.end as i64))?;
                    *self = Scope::AllBitField(range);
                    self.write_into_field(buffer, is_opt, is_present)
                } else {
                    *calls_until_ext_bitfield = calls_until_ext_bitfield.saturating_sub(1);
                    if let Some(range) = opt_bit_field {
                        if is_opt {
                            let result = buffer
                                .with_write_position_at(range.start, |b| b.write_bit(is_present));
                            range.start += 1;
                            result
                        } else {
                            Ok(())
                        }
                    } else {
                        Ok(())
                    }
                }
            }
        }
    }

    #[inline]
    pub fn read_from_field(
        &mut self,
        buffer: &mut BitBuffer,
        is_opt: bool,
    ) -> Result<Option<bool>, Error> {
        match self {
            Scope::OptBitField(range) => {
                if is_opt {
                    let result =
                        buffer.with_read_position_at(range.start, |buffer| buffer.read_bit());
                    range.start += 1;
                    Some(result).transpose()
                } else {
                    Ok(None)
                }
            }
            Scope::AllBitField(range) => {
                let result = buffer.with_read_position_at(range.start, |buffer| buffer.read_bit());
                range.start += 1;
                Some(result).transpose()
            }
            Scope::ExtensibleSequence {
                opt_bit_field,
                calls_until_ext_bitfield,
                number_of_ext_fields,
            } => {
                if *calls_until_ext_bitfield == 0 {
                    let read_number_of_ext_fields =
                        buffer.read_normally_small_length()? as usize + 1;
                    if read_number_of_ext_fields != *number_of_ext_fields {
                        return Err(Error::UnsupportedOperation(format!(
                            "Expected {} extended fields but got {}",
                            number_of_ext_fields, read_number_of_ext_fields
                        )));
                    }
                    let range = buffer.read_position..buffer.read_position + *number_of_ext_fields;
                    buffer.read_position = range.end; // skip bit-field
                    *self = Scope::AllBitField(range);
                    self.read_from_field(buffer, is_opt)
                } else {
                    *calls_until_ext_bitfield = calls_until_ext_bitfield.saturating_sub(1);
                    opt_bit_field
                        .as_mut()
                        .filter(|_| is_opt)
                        .map(|range| {
                            let result = buffer
                                .with_read_position_at(range.start, |buffer| buffer.read_bit());
                            range.start += 1;
                            result
                        })
                        .transpose()
                }
            }
        }
    }
}

#[derive(Default)]
pub struct UperWriter {
    buffer: BitBuffer,
    scope: Option<Scope>,
}

impl UperWriter {
    pub fn with_capacity(capacity_bytes: usize) -> Self {
        Self {
            buffer: BitBuffer::with_capacity(capacity_bytes),
            ..Default::default()
        }
    }

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
    pub fn scope_pushed<R, F: FnOnce(&mut Self) -> R>(&mut self, scope: Scope, f: F) -> R {
        let original = core::mem::replace(&mut self.scope, Some(scope));
        let result = f(self);
        let scope = core::mem::replace(&mut self.scope, original);
        // save because this is the original from above
        debug_assert!(scope.unwrap().exhausted());
        result
    }

    #[inline]
    pub fn scope_stashed<R, F: FnOnce(&mut Self) -> R>(&mut self, f: F) -> R {
        let scope = self.scope.take();
        let result = f(self);
        self.scope = scope;
        result
    }

    #[inline]
    pub fn write_bit_field_entry(&mut self, is_opt: bool, is_present: bool) -> Result<(), Error> {
        if let Some(scope) = &mut self.scope {
            scope.write_into_field(&mut self.buffer, is_opt, is_present)
        } else if is_opt {
            self.buffer.write_bit(is_present)
        } else {
            Ok(())
        }
    }

    #[inline]
    #[allow(clippy::redundant_pattern_matching)] // allow for const_*!
    pub fn with_buffer<T, F: FnOnce(&mut Self) -> Result<T, Error>>(
        &mut self,
        f: F,
    ) -> Result<T, Error> {
        if const_map_or!(self.scope, Scope::encode_as_open_type_field, false) {
            let mut writer = UperWriter::with_capacity(512);
            let result = f(&mut writer)?;
            self.buffer
                .write_octetstring(None, None, false, writer.buffer.content())?;
            Ok(result)
        } else {
            f(self)
        }
    }
}

impl Writer for UperWriter {
    type Error = Error;

    #[inline]
    fn write_sequence<C: sequence::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            if let Some(extension_after) = C::EXTENDED_AFTER_FIELD {
                w.buffer.write_bit(C::FIELD_COUNT > extension_after)?;
            }

            // In UPER the values for all OPTIONAL flags are written before any field
            // value is written. This remembers their position, so a later call of `write_opt`
            // can write them to the buffer
            let write_pos = w.buffer.write_position;
            let range = write_pos..write_pos + C::STD_OPTIONAL_FIELDS as usize;
            for _ in 0..C::STD_OPTIONAL_FIELDS {
                // insert in reverse order so that a simple pop() in `write_opt` retrieves
                // the relevant position
                if let Err(e) = w.buffer.write_bit(false) {
                    w.buffer.write_position = write_pos; // undo write_bits
                    return Err(e);
                }
            }

            if let Some(extension_after) = C::EXTENDED_AFTER_FIELD {
                w.scope_pushed(
                    Scope::ExtensibleSequence {
                        opt_bit_field: Some(range),
                        calls_until_ext_bitfield: (extension_after + 1) as usize,
                        number_of_ext_fields: (C::FIELD_COUNT - (extension_after + 1)) as usize,
                    },
                    f,
                )
            } else {
                w.scope_pushed(Scope::OptBitField(range), f)
            }
        })
    }

    #[inline]
    #[allow(clippy::redundant_pattern_matching)] // allow for const_*!
    fn write_sequence_of<C: sequenceof::Constraint, T: WritableType>(
        &mut self,
        slice: &[T::Type],
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        const MAX: u64 = i64::MAX as u64;
        let min = const_unwrap_or!(C::MIN, 0) as usize;
        let max = const_unwrap_or!(C::MAX, MAX) as usize;
        if slice.len() < min || slice.len() > max {
            return Err(Error::SizeNotInRange(
                slice.len() as u64,
                min as u64,
                max as u64,
            ));
        }
        self.scope_stashed(|w| {
            w.buffer
                .write_length_determinant(C::MIN, C::MAX, slice.len() as u64)?;
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
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            w.buffer.write_enumeration_index(
                C::STD_VARIANT_COUNT,
                C::EXTENSIBLE,
                enumerated.to_choice_index(),
            )
        })
    }

    #[inline]
    fn write_choice<C: choice::Constraint>(&mut self, choice: &C) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.scope_stashed(|w| {
            let index = choice.to_choice_index();

            // this fails if the index is out of range
            w.buffer
                .write_choice_index(C::STD_VARIANT_COUNT, C::EXTENSIBLE, index)?;

            if index >= C::STD_VARIANT_COUNT {
                // TODO performance
                let mut writer = UperWriter::with_capacity(512);
                choice.write_content(&mut writer)?;
                w.buffer
                    .write_octetstring(None, None, false, writer.byte_content())
            } else {
                choice.write_content(w)
            }
        })
    }

    #[inline]
    #[allow(clippy::redundant_pattern_matching)] // allow for const_*!
    fn write_opt<T: WritableType>(
        &mut self,
        value: Option<&<T as WritableType>::Type>,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(true, const_is_some!(value))?;
        if let Some(value) = value {
            self.scope_stashed(|w| T::write_value(w, value))
        } else {
            Ok(())
        }
    }

    #[inline]
    #[allow(clippy::redundant_pattern_matching)] // allow for const_*!
    fn write_number<T: numbers::Number, C: numbers::Constraint<T>>(
        &mut self,
        value: T,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        let value = value.to_i64();

        let max_fn = if C::EXTENSIBLE {
            let min = const_unwrap_or!(C::MIN, 0);
            let max = const_unwrap_or!(C::MAX, i64::MAX);
            value < min || value > max
        } else {
            const_is_none!(C::MIN) && const_is_none!(C::MAX)
        };

        if max_fn {
            self.with_buffer(|w| {
                if C::EXTENSIBLE {
                    w.buffer.write_bit(true)?;
                }
                w.buffer.write_unconstrained_whole_number(value)
            })
        } else {
            self.with_buffer(|w| {
                if C::EXTENSIBLE {
                    w.buffer.write_bit(false)?;
                }
                w.buffer.write_constrained_whole_number(
                    const_unwrap_or!(C::MIN, 0),
                    const_unwrap_or!(C::MAX, i64::MAX),
                    value,
                )
            })
        }
    }

    #[inline]
    #[allow(clippy::redundant_pattern_matching)] // allow for const_*!
    fn write_utf8string<C: utf8string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            // ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 30.3
            // For 'known-multiplier character string types' there is no min/max in the encoding
            if !C::EXTENSIBLE {
                let chars = value.chars().count() as u64;
                let min = const_unwrap_or!(C::MIN, 0);
                let max = const_unwrap_or!(C::MAX, u64::MAX);
                if chars < min || chars > max {
                    return Err(Error::SizeNotInRange(chars, min, max));
                }
            }
            w.buffer
                .write_octetstring(None, None, false, value.as_bytes())
        })
    }

    #[inline]
    fn write_octet_string<C: octetstring::Constraint>(
        &mut self,
        value: &[u8],
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            w.buffer
                .write_octetstring(C::MIN, C::MAX, C::EXTENSIBLE, value)
        })
    }

    #[inline]
    fn write_bit_string<C: bitstring::Constraint>(
        &mut self,
        value: &[u8],
        bit_len: u64,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            w.buffer
                .write_bitstring(C::MIN, C::MAX, C::EXTENSIBLE, value, 0, bit_len)
        })
    }

    #[inline]
    fn write_boolean<C: boolean::Constraint>(&mut self, value: bool) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| w.buffer.write_bit(value))
    }
}

pub struct UperReader {
    buffer: BitBuffer,
    scope: Option<Scope>,
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
    pub fn scope_pushed<R, F: FnOnce(&mut Self) -> R>(&mut self, scope: Scope, f: F) -> R {
        let original = core::mem::replace(&mut self.scope, Some(scope));
        let result = f(self);
        let scope = core::mem::replace(&mut self.scope, original);
        let scope = scope.unwrap(); // save because this is the original from above
        debug_assert!(scope.exhausted());
        result
    }

    #[inline]
    pub fn scope_stashed<R, F: FnOnce(&mut Self) -> R>(&mut self, f: F) -> R {
        let scope = self.scope.take();
        let result = f(self);
        self.scope = scope;
        result
    }

    #[inline]
    pub fn read_whole_sub_slice<T, E, F: FnOnce(&mut Self) -> Result<T, E>>(
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

    #[inline]
    pub fn read_bit_field_entry(&mut self, is_opt: bool) -> Result<Option<bool>, Error> {
        if let Some(scope) = &mut self.scope {
            scope.read_from_field(&mut self.buffer, is_opt)
        } else if is_opt {
            Some(self.buffer.read_bit()).transpose()
        } else {
            Ok(None)
        }
    }

    #[inline]
    pub fn with_buffer<T, F: FnOnce(&mut Self) -> Result<T, Error>>(
        &mut self,
        f: F,
    ) -> Result<T, Error> {
        if self
            .scope
            .as_ref()
            .map(Scope::encode_as_open_type_field)
            .unwrap_or(false)
        {
            let len = self.buffer.read_length_determinant(None, None)?;
            self.read_whole_sub_slice(len as usize, f)
        } else {
            f(self)
        }
    }

    pub fn reset_read_position(&mut self) {
        self.buffer.reset_read_position()
    }
}

impl Reader for UperReader {
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
        let _ = self.read_bit_field_entry(false);
        self.with_buffer(|r| {
            if let Some(extension_after) = C::EXTENDED_AFTER_FIELD {
                let has_extension = r.buffer.read_bit()?;
                let expects_extension = C::FIELD_COUNT > extension_after;
                if has_extension != expects_extension {
                    return Err(Error::InvalidExtensionConstellation(
                        expects_extension,
                        has_extension,
                    ));
                }
            }

            // In UPER the values for all OPTIONAL flags are written before any field
            // value is written. This remembers their position, so a later call of `read_opt`
            // can retrieve them from the buffer
            let range =
                r.buffer.read_position..r.buffer.read_position + C::STD_OPTIONAL_FIELDS as usize;
            if r.buffer.bit_len() < range.end {
                return Err(Error::EndOfStream);
            }
            r.buffer.read_position = range.end; // skip optional

            if let Some(extension_after) = C::EXTENDED_AFTER_FIELD {
                r.scope_pushed(
                    Scope::ExtensibleSequence {
                        opt_bit_field: Some(range),
                        calls_until_ext_bitfield: (extension_after + 1) as usize,
                        number_of_ext_fields: (C::FIELD_COUNT - (extension_after + 1)) as usize,
                    },
                    f,
                )
            } else {
                r.scope_pushed(Scope::OptBitField(range), f)
            }
        })
    }

    #[inline]
    fn read_sequence_of<C: sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<T::Type>, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| {
            let min = const_unwrap_or!(C::MIN, 0);
            let max = const_unwrap_or!(C::MAX, u64::MAX);
            let len = r.buffer.read_length_determinant(None, None)? + min;
            if len > max {
                Err(Error::SizeNotInRange(len, min, max))
            } else {
                r.scope_stashed(|w| {
                    let mut vec = Vec::with_capacity(len as usize);
                    for _ in 0..len {
                        vec.push(T::read_value(w)?);
                    }
                    Ok(vec)
                })
            }
        })
    }

    #[inline]
    fn read_enumerated<C: enumerated::Constraint>(&mut self) -> Result<C, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| {
            r.buffer
                .read_enumeration_index(C::STD_VARIANT_COUNT, C::EXTENSIBLE)
        })
        .and_then(|index| {
            C::from_choice_index(index)
                .ok_or_else(|| Error::InvalidChoiceIndex(index, C::VARIANT_COUNT))
        })
    }

    #[inline]
    fn read_choice<C: choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.scope_stashed(|r| {
            let index = r
                .buffer
                .read_choice_index(C::STD_VARIANT_COUNT, C::EXTENSIBLE)?;
            if index >= C::STD_VARIANT_COUNT {
                let length = r.buffer.read_length_determinant(None, None)?;
                r.read_whole_sub_slice(length as usize, |r| Ok((index, C::read_content(index, r)?)))
            } else {
                Ok((index, C::read_content(index, r)?))
            }
            .and_then(|(index, content)| {
                content.ok_or_else(|| Error::InvalidChoiceIndex(index, C::VARIANT_COUNT))
            })
        })
    }

    #[inline]
    fn read_opt<T: ReadableType>(
        &mut self,
    ) -> Result<Option<<T as ReadableType>::Type>, Self::Error> {
        // unwrap: as opt-field this must and will return some value
        if self.read_bit_field_entry(true)?.unwrap() {
            self.scope_stashed(T::read_value).map(Some)
        } else {
            Ok(None)
        }
    }

    #[inline]
    #[allow(clippy::redundant_pattern_matching)] // allow for const_*!
    fn read_number<T: numbers::Number, C: numbers::Constraint<T>>(
        &mut self,
    ) -> Result<T, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| {
            let unconstrained = if C::EXTENSIBLE {
                r.buffer.read_bit()?
            } else {
                const_is_none!(C::MIN) && const_is_none!(C::MAX)
            };

            if unconstrained {
                r.buffer.read_unconstrained_whole_number().map(T::from_i64)
            } else {
                r.buffer
                    .read_constrained_whole_number(
                        const_unwrap_or!(C::MIN, 0),
                        const_unwrap_or!(C::MAX, i64::MAX),
                    )
                    .map(T::from_i64)
            }
        })
    }

    #[inline]
    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| {
            // ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 30.3
            // For 'known-multiplier character string types' there is no min/max in the encoding
            let octets = r.buffer.read_octetstring(None, None, false)?;
            String::from_utf8(octets).map_err(|_| Self::Error::InvalidUtf8String)
        })
    }

    #[inline]
    fn read_octet_string<C: octetstring::Constraint>(&mut self) -> Result<Vec<u8>, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| r.buffer.read_octetstring(C::MIN, C::MAX, C::EXTENSIBLE))
    }

    #[inline]
    fn read_bit_string<C: bitstring::Constraint>(&mut self) -> Result<(Vec<u8>, u64), Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| r.buffer.read_bitstring(C::MIN, C::MAX, C::EXTENSIBLE))
    }

    #[inline]
    fn read_boolean<C: boolean::Constraint>(&mut self) -> Result<bool, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| r.buffer.read_boolean())
    }
}
