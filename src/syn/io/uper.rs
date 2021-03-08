use crate::io::per::err::Error;
use crate::io::per::unaligned::buffer::BitBuffer;
use crate::io::per::unaligned::BitWrite;
use crate::io::per::unaligned::BYTE_LEN;
use crate::io::per::PackedRead;
use crate::io::per::PackedWrite;
use crate::syn::*;
use std::ops::Range;

pub use crate::io::per::unaligned::buffer::Bits;
pub use crate::io::per::unaligned::ScopedBitRead;
use crate::model::Charset;

/// This ist enum is the main reason, the new impl is about ~10% slower (2020-09) than the previous/
/// legacy implementation. This dynamic state tracking at runtime could be avoided by passing all
/// values as const generics on each `read_`*/`write_`* call [RFC 2000]. Maybe getting rid of all
/// `mem::replace` calls would also be sufficient.
///
/// [RFC 2000]: https://github.com/rust-lang/rust/issues/44580    
pub enum Scope {
    OptBitField(Range<usize>),
    AllBitField(Range<usize>),
    /// According to ITU-T X.691 | ISO/IEC 8825-2:2015, an extensible struct is built as
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
        bits: &mut impl ScopedBitRead,
        is_opt: bool,
    ) -> Result<Option<bool>, Error> {
        match self {
            Scope::OptBitField(range) => {
                if is_opt {
                    let result =
                        bits.with_read_position_at(range.start, |buffer| buffer.read_bit());
                    range.start += 1;
                    Some(result).transpose()
                } else {
                    Ok(None)
                }
            }
            Scope::AllBitField(range) => {
                let result = bits.with_read_position_at(range.start, |buffer| buffer.read_bit());
                range.start += 1;
                Some(result).transpose()
            }
            Scope::ExtensibleSequence {
                opt_bit_field,
                calls_until_ext_bitfield,
                number_of_ext_fields,
            } => {
                if *calls_until_ext_bitfield == 0 {
                    let read_number_of_ext_fields = bits.read_normally_small_length()? as usize + 1;
                    if read_number_of_ext_fields != *number_of_ext_fields {
                        return Err(Error::UnsupportedOperation(format!(
                            "Expected {} extended fields but got {}",
                            number_of_ext_fields, read_number_of_ext_fields
                        )));
                    }
                    let range = bits.pos()..bits.pos() + *number_of_ext_fields;
                    bits.set_pos(range.end); // skip bit-field
                    *self = Scope::AllBitField(range);
                    self.read_from_field(bits, is_opt)
                } else {
                    *calls_until_ext_bitfield = calls_until_ext_bitfield.saturating_sub(1);
                    opt_bit_field
                        .as_mut()
                        .filter(|_| is_opt)
                        .map(|range| {
                            let result =
                                bits.with_read_position_at(range.start, |buffer| buffer.read_bit());
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
    bits: BitBuffer,
    scope: Option<Scope>,
}

impl UperWriter {
    pub fn with_capacity(capacity_bytes: usize) -> Self {
        Self {
            bits: BitBuffer::with_capacity(capacity_bytes),
            ..Default::default()
        }
    }

    pub fn byte_content(&self) -> &[u8] {
        self.bits.content()
    }

    pub const fn bit_len(&self) -> usize {
        self.bits.bit_len()
    }

    pub fn into_bytes_vec(self) -> Vec<u8> {
        debug_assert_eq!(
            (self.bit_len() + BYTE_LEN - 1) / BYTE_LEN,
            self.bits.buffer.len()
        );
        self.bits.into()
    }

    pub fn as_reader(&self) -> UperReader<Bits> {
        UperReader::from(Bits::from((self.byte_content(), self.bit_len())))
    }

    #[inline]
    pub fn scope_pushed<R, F: FnOnce(&mut Self) -> R>(&mut self, scope: Scope, f: F) -> R {
        let original = core::mem::replace(&mut self.scope, Some(scope));
        let result = f(self);
        let scope = core::mem::replace(&mut self.scope, original);
        // save because this is supposed to be the original from above
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
            scope.write_into_field(&mut self.bits, is_opt, is_present)
        } else if is_opt {
            self.bits.write_bit(is_present)
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
            self.bits
                .write_octetstring(None, None, false, writer.bits.content())?;
            Ok(result)
        } else {
            f(self)
        }
    }

    #[inline]
    pub fn write_extensible_bit_and_length_or_err(
        &mut self,
        extensible: bool,
        min: Option<u64>,
        max: Option<u64>,
        upper_limit: u64,
        len: u64,
    ) -> Result<bool, Error> {
        let unwrapped_min = const_unwrap_or!(min, 0);
        let unwrapped_max = const_unwrap_or!(max, upper_limit);
        let out_of_range = len < unwrapped_min || len > unwrapped_max;

        if extensible {
            self.bits.write_bit(out_of_range)?;
        }

        if out_of_range {
            if !extensible {
                return Err(Error::SizeNotInRange(len, unwrapped_min, unwrapped_max));
            } else {
                self.bits.write_length_determinant(None, None, len)?;
            }
        } else {
            self.bits.write_length_determinant(min, max, len)?;
        }

        Ok(out_of_range)
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
                w.bits.write_bit(C::FIELD_COUNT > extension_after)?;
            }

            // In UPER the values for all OPTIONAL flags are written before any field
            // value is written. This remembers their position, so a later call of `write_opt`
            // can write them to the buffer
            let write_pos = w.bits.write_position;
            let range = write_pos..write_pos + C::STD_OPTIONAL_FIELDS as usize;
            for _ in 0..C::STD_OPTIONAL_FIELDS {
                // insert in reverse order so that a simple pop() in `write_opt` retrieves
                // the relevant position
                if let Err(e) = w.bits.write_bit(false) {
                    w.bits.write_position = write_pos; // undo write_bits
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
        self.scope_stashed(|w| {
            w.write_extensible_bit_and_length_or_err(
                C::EXTENSIBLE,
                C::MIN,
                C::MAX,
                i64::MAX as u64,
                slice.len() as u64,
            )?;

            w.scope_stashed(|w| {
                for value in slice {
                    T::write_value(w, value)?;
                }
                Ok(())
            })
        })
    }

    #[inline]
    fn write_set<C: set::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error> {
        self.write_sequence::<C, F>(f)
    }

    #[inline]
    fn write_set_of<C: setof::Constraint, T: WritableType>(
        &mut self,
        slice: &[<T as WritableType>::Type],
    ) -> Result<(), Self::Error> {
        self.write_sequence_of::<C, T>(slice)
    }

    #[inline]
    fn write_enumerated<C: enumerated::Constraint>(
        &mut self,
        enumerated: &C,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            w.bits.write_enumeration_index(
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
            w.bits
                .write_choice_index(C::STD_VARIANT_COUNT, C::EXTENSIBLE, index)?;

            if index >= C::STD_VARIANT_COUNT {
                // TODO performance
                let mut writer = UperWriter::with_capacity(512);
                choice.write_content(&mut writer)?;
                w.bits
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
                    w.bits.write_bit(true)?;
                }
                w.bits.write_unconstrained_whole_number(value)
            })
        } else {
            self.with_buffer(|w| {
                if C::EXTENSIBLE {
                    w.bits.write_bit(false)?;
                }
                w.bits.write_constrained_whole_number(
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
            if !C::EXTENSIBLE {
                let chars = value.chars().count() as u64;
                let min = const_unwrap_or!(C::MIN, 0);
                let max = const_unwrap_or!(C::MAX, u64::MAX);
                if chars < min || chars > max {
                    return Err(Error::SizeNotInRange(chars, min, max));
                }
            }

            // ITU-T X.691 | ISO/IEC 8825-2:2015, chapter 30.3
            // For 'known-multiplier character string types' there is no min/max in the encoding
            w.bits
                .write_octetstring(None, None, false, value.as_bytes())
        })
    }

    #[inline]
    fn write_ia5string<C: ia5string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            Error::ensure_string_valid(Charset::Ia5, value)?;

            w.write_extensible_bit_and_length_or_err(
                C::EXTENSIBLE,
                C::MIN,
                C::MAX,
                u64::MAX,
                value.chars().count() as u64,
            )?;

            for char in value.chars().map(|c| c as u8) {
                // 7 bits
                w.bits.write_bits_with_offset(&[char], 1)?;
            }

            Ok(())
        })
    }

    #[inline]
    fn write_numeric_string<C: numericstring::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            Error::ensure_string_valid(Charset::Numeric, value)?;

            w.write_extensible_bit_and_length_or_err(
                C::EXTENSIBLE,
                C::MIN,
                C::MAX,
                u64::MAX,
                value.chars().count() as u64,
            )?;

            for char in value.chars().map(|c| c as u8) {
                let char = match char - 32 {
                    0 => 0,
                    c => c - 15,
                };
                w.bits.write_bits_with_offset(&[char], 4)?;
            }

            Ok(())
        })
    }

    #[inline]
    fn write_octet_string<C: octetstring::Constraint>(
        &mut self,
        value: &[u8],
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            w.bits
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
            w.bits
                .write_bitstring(C::MIN, C::MAX, C::EXTENSIBLE, value, 0, bit_len)
        })
    }

    #[inline]
    fn write_boolean<C: boolean::Constraint>(&mut self, value: bool) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| w.bits.write_bit(value))
    }
}

pub struct UperReader<B: ScopedBitRead> {
    bits: B,
    scope: Option<Scope>,
}

/*
impl<B: ScopedBitRead> From<B> for UperReader<B> {
    fn from(bits: B) -> Self {
        UperReader { bits, scope: None }
    }
}*/

impl<'a, I: Into<Bits<'a>>> From<I> for UperReader<Bits<'a>> {
    fn from(bits: I) -> Self {
        UperReader {
            bits: bits.into(),
            scope: None,
        }
    }
}

impl<B: ScopedBitRead> UperReader<B> {
    #[inline]
    pub fn bits_remaining(&self) -> usize {
        self.bits.remaining()
    }

    #[inline]
    pub fn scope_pushed<R, F: FnOnce(&mut Self) -> R>(&mut self, scope: Scope, f: F) -> R {
        let original = core::mem::replace(&mut self.scope, Some(scope));
        let result = f(self);
        if cfg!(debug_assertions) {
            let scope = core::mem::replace(&mut self.scope, original);
            // save because this is the original from above
            debug_assert!(scope.unwrap().exhausted());
        } else {
            self.scope = original;
        }
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
        let write_position = self.bits.pos() + (length_bytes * BYTE_LEN);
        let write_original = core::mem::replace(&mut self.bits.len(), write_position);
        let result = f(self);
        // extend to original position
        let len = self.bits.set_len(write_original);
        debug_assert_eq!(write_original, len);
        if result.is_ok() {
            // on successful read, skip the slice
            self.bits.set_pos(write_position);
        }
        result
    }

    #[inline]
    pub fn read_bit_field_entry(&mut self, is_opt: bool) -> Result<Option<bool>, Error> {
        if let Some(scope) = &mut self.scope {
            scope.read_from_field(&mut self.bits, is_opt)
        } else if is_opt {
            Some(self.bits.read_bit()).transpose()
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
            let len = self.bits.read_length_determinant(None, None)?;
            self.read_whole_sub_slice(len as usize, f)
        } else {
            f(self)
        }
    }
}

impl<B: ScopedBitRead> Reader for UperReader<B> {
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
                let has_extension = r.bits.read_bit()?;
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
            if r.bits.remaining() < C::STD_OPTIONAL_FIELDS as usize {
                return Err(Error::EndOfStream);
            }
            let range = r.bits.pos()..r.bits.pos() + C::STD_OPTIONAL_FIELDS as usize;
            r.bits.set_pos(range.end); // skip optional

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
            let len = if C::EXTENSIBLE && r.bits.read_bit()? {
                r.bits.read_length_determinant(None, None)?
            } else {
                r.bits.read_length_determinant(C::MIN, C::MAX)?
            };
            r.scope_stashed(|r| {
                let mut vec = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    vec.push(T::read_value(r)?);
                }
                Ok(vec)
            })
        })
    }

    #[inline]
    fn read_set<C: set::Constraint, S: Sized, F: Fn(&mut Self) -> Result<S, Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<S, Self::Error> {
        self.read_sequence::<C, S, F>(f)
    }

    #[inline]
    fn read_set_of<C: setof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<<T as ReadableType>::Type>, Self::Error> {
        self.read_sequence_of::<C, T>()
    }

    #[inline]
    fn read_enumerated<C: enumerated::Constraint>(&mut self) -> Result<C, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| {
            r.bits
                .read_enumeration_index(C::STD_VARIANT_COUNT, C::EXTENSIBLE)
        })
        .and_then(|index| {
            C::from_choice_index(index).ok_or(Error::InvalidChoiceIndex(index, C::VARIANT_COUNT))
        })
    }

    #[inline]
    fn read_choice<C: choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.scope_stashed(|r| {
            let index = r
                .bits
                .read_choice_index(C::STD_VARIANT_COUNT, C::EXTENSIBLE)?;
            if index >= C::STD_VARIANT_COUNT {
                let length = r.bits.read_length_determinant(None, None)?;
                r.read_whole_sub_slice(length as usize, |r| Ok((index, C::read_content(index, r)?)))
            } else {
                Ok((index, C::read_content(index, r)?))
            }
            .and_then(|(index, content)| {
                content.ok_or(Error::InvalidChoiceIndex(index, C::VARIANT_COUNT))
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
                r.bits.read_bit()?
            } else {
                const_is_none!(C::MIN) && const_is_none!(C::MAX)
            };

            if unconstrained {
                r.bits.read_unconstrained_whole_number().map(T::from_i64)
            } else {
                r.bits
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
            // ITU-T X.691 | ISO/IEC 8825-2:2015, chapter 30.3
            // For 'known-multiplier character string types' there is no min/max in the encoding
            let octets = r.bits.read_octetstring(None, None, false)?;
            String::from_utf8(octets).map_err(Self::Error::FromUtf8Error)
        })
    }

    #[inline]
    fn read_ia5string<C: ia5string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| {
            let len = if C::EXTENSIBLE && r.bits.read_bit()? {
                r.bits.read_length_determinant(None, None)?
            } else {
                r.bits.read_length_determinant(C::MIN, C::MAX)?
            };

            let mut buffer = vec![0u8; len as usize];
            for i in 0..len as usize {
                r.bits.read_bits_with_offset(&mut buffer[i..i + 1], 1)?;
            }

            String::from_utf8(buffer).map_err(Self::Error::FromUtf8Error)
        })
    }

    #[inline]
    fn read_numeric_string<C: numericstring::Constraint>(&mut self) -> Result<String, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| {
            let len = if C::EXTENSIBLE && r.bits.read_bit()? {
                r.bits.read_length_determinant(None, None)?
            } else {
                r.bits.read_length_determinant(C::MIN, C::MAX)?
            };

            let mut buffer = vec![0u8; len as usize];
            for i in 0..len as usize {
                r.bits.read_bits_with_offset(&mut buffer[i..i + 1], 4)?;
                match buffer[i] {
                    0_u8 => buffer[i] = 32_u8,
                    c => buffer[i] = 32_u8 + 15 + c,
                }
            }

            String::from_utf8(buffer).map_err(Self::Error::FromUtf8Error)
        })
    }

    #[inline]
    fn read_octet_string<C: octetstring::Constraint>(&mut self) -> Result<Vec<u8>, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| r.bits.read_octetstring(C::MIN, C::MAX, C::EXTENSIBLE))
    }

    #[inline]
    fn read_bit_string<C: bitstring::Constraint>(&mut self) -> Result<(Vec<u8>, u64), Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| r.bits.read_bitstring(C::MIN, C::MAX, C::EXTENSIBLE))
    }

    #[inline]
    fn read_boolean<C: boolean::Constraint>(&mut self) -> Result<bool, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| r.bits.read_boolean())
    }
}

pub trait UperDecodable<'a, I: Into<Bits<'a>> + 'a> {
    fn decode_from_uper(bits: I) -> Result<Self, Error>
    where
        Self: Sized;
}

impl<'a, R: Readable, I: Into<Bits<'a>> + 'a> UperDecodable<'a, I> for R {
    fn decode_from_uper(bits: I) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let mut reader = UperReader::from(bits);
        Self::read(&mut reader)
    }
}
