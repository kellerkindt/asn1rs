use crate::io::per::err::Error;
use crate::io::per::err::ErrorKind;
use crate::io::per::unaligned::buffer::BitBuffer;
use crate::io::per::unaligned::BitWrite;
use crate::io::per::unaligned::BYTE_LEN;
use crate::io::per::PackedRead;
use crate::io::per::PackedWrite;
use crate::model::Charset;
use crate::syn::*;
use std::fmt::Debug;
use std::ops::Range;

pub use crate::io::per::unaligned::buffer::Bits;
pub use crate::io::per::unaligned::ScopedBitRead;

#[derive(Debug, Clone)]
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
        name: &'static str,
        bit_pos: usize,
        opt_bit_field: Option<Range<usize>>,
        calls_until_ext_bitfield: usize,
        number_of_ext_fields: usize,
    },
    /// Indicates that the extensible sequence has no extension body
    ExtensibleSequenceEmpty(&'static str),
}

impl Scope {
    #[inline]
    pub const fn exhausted(&self) -> bool {
        match self {
            Scope::OptBitField(range) => range.start == range.end,
            Scope::AllBitField(range) => range.start == range.end,
            Scope::ExtensibleSequence {
                name: _,
                bit_pos: _,
                opt_bit_field,
                calls_until_ext_bitfield: _,
                number_of_ext_fields: _,
            } => match opt_bit_field {
                Some(range) => range.start == range.end,
                None => true,
            },
            Scope::ExtensibleSequenceEmpty(_) => true,
        }
    }

    #[inline]
    pub const fn encode_as_open_type_field(&self) -> bool {
        matches!(
            self,
            Scope::AllBitField(_) | Scope::ExtensibleSequenceEmpty(_)
        )
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
                name,
                bit_pos: ext_bit_pos,
                opt_bit_field,
                calls_until_ext_bitfield,
                number_of_ext_fields,
            } => {
                if *calls_until_ext_bitfield == 0 {
                    buffer.with_write_position_at(*ext_bit_pos, |b| b.write_bit(is_present))?;
                    if is_present {
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

                        // pos + 1 because the bit for the current call is already set
                        // by the initializer loop above
                        let range = pos + 1..buffer.write_position;
                        *self = Scope::AllBitField(range);
                    } else {
                        *self = Scope::ExtensibleSequenceEmpty(name);
                    }
                    // no need for this
                    // if is_present is true, the bit is already set (initialize loop above)
                    // if is_present is false, no bit will be written (empty)
                    // self.write_into_field(buffer, is_opt, is_present)
                    Ok(())
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
            Scope::ExtensibleSequenceEmpty(name) => {
                if is_present {
                    Err(ErrorKind::ExtensionFieldsInconsistent(name.to_string()).into())
                } else {
                    Ok(())
                }
            }
        }
    }

    #[inline]
    pub fn read_from_field(
        &mut self,
        #[cfg(feature = "descriptive-deserialize-errors")] descriptions: &mut Vec<ScopeDescription>,
        bits: &mut impl ScopedBitRead,
        is_opt: bool,
    ) -> Result<Option<bool>, Error> {
        match self {
            Scope::OptBitField(range) => {
                if range.start >= range.end {
                    Ok(Some(false))
                } else if is_opt {
                    let result =
                        bits.with_read_position_at(range.start, |buffer| buffer.read_bit());
                    range.start += 1;
                    Some(result).transpose()
                } else {
                    Ok(None)
                }
            }
            Scope::AllBitField(range) => {
                if range.start < range.end {
                    let result =
                        bits.with_read_position_at(range.start, |buffer| buffer.read_bit());
                    range.start += 1;
                    Some(result).transpose()
                } else {
                    // all further extensible fields are not present
                    Ok(Some(false))
                }
            }
            Scope::ExtensibleSequence {
                name,
                bit_pos: ext_bit_pos,
                opt_bit_field,
                calls_until_ext_bitfield,
                number_of_ext_fields,
            } => {
                if *calls_until_ext_bitfield == 0 {
                    if bits.with_read_position_at(*ext_bit_pos, |b| b.read_bit())? {
                        let read_number_of_ext_fields =
                            bits.read_normally_small_length()? as usize + 1;
                        if read_number_of_ext_fields > *number_of_ext_fields {
                            #[cfg(feature = "descriptive-deserialize-errors")]
                            descriptions.push(ScopeDescription::warning(
                                format!("read_number_of_ext_fields({read_number_of_ext_fields}) > *number_of_ext_fields({number_of_ext_fields})")
                            ));
                            //     return Err(Error::UnsupportedOperation(format!(
                            //         "Expected no more than {} extended field{} but got {}",
                            //         number_of_ext_fields,
                            //         if *number_of_ext_fields != 1 { "s" } else { "" },
                            //         read_number_of_ext_fields
                            //     )));
                        }
                        let range = bits.pos()..bits.pos() + *number_of_ext_fields;
                        bits.set_pos(range.start + read_number_of_ext_fields); // skip bit-field
                        *self = Scope::AllBitField(range);
                    } else {
                        *self = Scope::ExtensibleSequenceEmpty(name);
                    }
                    self.read_from_field(
                        #[cfg(feature = "descriptive-deserialize-errors")]
                        descriptions,
                        bits,
                        is_opt,
                    )
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
            Scope::ExtensibleSequenceEmpty(_) => Ok(Some(false)),
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
    pub fn scope_pushed<T, E, F: FnOnce(&mut Self) -> Result<T, E>>(
        &mut self,
        scope: Scope,
        f: F,
    ) -> Result<T, E> {
        let original = core::mem::replace(&mut self.scope, Some(scope));
        let result = f(self);
        if cfg!(debug_assertions) && result.is_ok() {
            let scope = core::mem::replace(&mut self.scope, original);
            // call to .unwrap() is save because this is supposed to be the original from above
            debug_assert!(
                scope.clone().unwrap().exhausted(),
                "Not exhausted: {:?}",
                scope.unwrap()
            );
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
                return Err(ErrorKind::SizeNotInRange(len, unwrapped_min, unwrapped_max).into());
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
            let extension = if let Some(extension_after) = C::EXTENDED_AFTER_FIELD {
                let bit_pos = w.bits.write_position;
                // if no extension field is present, none will call into overwriting this
                w.bits.write_bit(false)?;
                Some((extension_after, bit_pos))
            } else {
                None
            };

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

            if let Some((extension_after, bit_pos)) = extension {
                w.scope_pushed(
                    Scope::ExtensibleSequence {
                        name: C::NAME,
                        bit_pos,
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
            self.with_buffer(|w| w.scope_stashed(|w| T::write_value(w, value)))
        } else {
            Ok(())
        }
    }

    #[inline]
    fn write_default<C: default::Constraint<Owned = T::Type>, T: WritableType>(
        &mut self,
        value: &T::Type,
    ) -> Result<(), Self::Error> {
        let present = C::DEFAULT_VALUE.ne(value);
        self.write_bit_field_entry(true, present)?;
        if present {
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
                    return Err(ErrorKind::SizeNotInRange(chars, min, max).into());
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
    fn write_printable_string<C: printablestring::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            Error::ensure_string_valid(Charset::Printable, value)?;

            w.write_extensible_bit_and_length_or_err(
                C::EXTENSIBLE,
                C::MIN,
                C::MAX,
                u64::MAX,
                value.chars().count() as u64,
            )?;

            for char in value.chars() {
                w.bits.write_bits_with_offset(&[char as u8], 1)?;
            }

            Ok(())
        })
    }

    #[inline]
    fn write_visible_string<C: visiblestring::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| {
            Error::ensure_string_valid(Charset::Visible, value)?;

            w.write_extensible_bit_and_length_or_err(
                C::EXTENSIBLE,
                C::MIN,
                C::MAX,
                u64::MAX,
                value.chars().count() as u64,
            )?;

            for char in value.chars() {
                w.bits.write_bits_with_offset(&[char as u8], 1)?;
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

    #[inline]
    fn write_null<C: null::Constraint>(&mut self, _value: &Null) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct UperReader<B: ScopedBitRead> {
    bits: B,
    scope: Option<Scope>,
    #[cfg(feature = "descriptive-deserialize-errors")]
    scope_description: Vec<ScopeDescription>,
}

impl<B: ScopedBitRead> From<B> for UperReader<B> {
    fn from(bits: B) -> Self {
        UperReader {
            bits,
            scope: None,
            #[cfg(feature = "descriptive-deserialize-errors")]
            scope_description: Vec::new(),
        }
    }
}

impl<'a> From<(&'a [u8], usize)> for UperReader<Bits<'a>> {
    fn from(bits: (&'a [u8], usize)) -> Self {
        UperReader::from(Bits::from(bits))
    }
}

impl<B: ScopedBitRead> UperReader<B> {
    #[inline]
    fn read_length_determinant(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
    ) -> Result<u64, Error> {
        #[allow(clippy::let_and_return)]
        let result = self.bits.read_length_determinant(lower_bound, upper_bound);
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::bits_length_determinant(
                lower_bound,
                upper_bound,
                result.clone(),
            ));
        result
    }

    #[inline]
    fn read_enumeration_index(
        &mut self,
        std_variants: u64,
        extensible: bool,
    ) -> Result<u64, Error> {
        #[allow(clippy::let_and_return)]
        let result = self.bits.read_enumeration_index(std_variants, extensible);
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::bits_enumeration_index(
                std_variants,
                extensible,
                result.clone(),
            ));
        result
    }

    #[inline]
    fn read_choice_index(&mut self, std_variants: u64, extensible: bool) -> Result<u64, Error> {
        #[allow(clippy::let_and_return)]
        let result = self.bits.read_choice_index(std_variants, extensible);
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::bits_choice_index(
                std_variants,
                extensible,
                result.clone(),
            ));
        result
    }

    #[inline]
    pub fn bits_remaining(&self) -> usize {
        self.bits.remaining()
    }

    #[inline]
    pub fn scope_pushed<T, F: FnOnce(&mut Self) -> Result<T, Error>>(
        &mut self,
        scope: Scope,
        f: F,
    ) -> Result<T, Error> {
        let original = core::mem::replace(&mut self.scope, Some(scope));
        let result = f(self);
        if cfg!(debug_assertions) && result.is_ok() {
            let scope = core::mem::replace(&mut self.scope, original);
            // call to .unwrap() is save because this is supposed to be the original from above
            debug_assert!(
                scope.clone().unwrap().exhausted(),
                "Not exhausted: {:?}",
                scope.unwrap()
            );
        } else {
            self.scope = original;
        }
        result
    }

    #[inline]
    pub fn scope_stashed<T, F: FnOnce(&mut Self) -> Result<T, Error>>(
        &mut self,
        f: F,
    ) -> Result<T, Error> {
        let scope = self.scope.take();
        let result = f(self);
        self.scope = scope;
        result
    }

    #[inline]
    pub fn read_whole_sub_slice<T, F: FnOnce(&mut Self) -> Result<T, Error>>(
        &mut self,
        length_bytes: usize,
        f: F,
    ) -> Result<T, Error> {
        let write_position = self.bits.pos() + (length_bytes * BYTE_LEN);
        let write_original = core::mem::replace(&mut self.bits.len(), write_position);
        let result = f(self);
        // extend to original position
        let len = self.bits.set_len(write_original);
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::read_whole_sub_slice(
                length_bytes,
                write_position,
                write_original,
                len,
                &result,
            ));
        debug_assert_eq!(write_original, len);
        if result.is_ok() {
            // on successful read, skip the slice
            self.bits.set_pos(write_position);
        }
        result
    }

    #[inline]
    pub fn read_bit_field_entry(&mut self, is_opt: bool) -> Result<Option<bool>, Error> {
        #[allow(clippy::let_and_return)]
        let result = if let Some(scope) = &mut self.scope {
            scope.read_from_field(
                #[cfg(feature = "descriptive-deserialize-errors")]
                &mut self.scope_description,
                &mut self.bits,
                is_opt,
            )
        } else if is_opt {
            Some(self.bits.read_bit()).transpose()
        } else {
            Ok(None)
        };

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::read_bit_field_entry(is_opt, &result));

        result
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
            let len = self.read_length_determinant(None, None)?;
            self.read_whole_sub_slice(len as usize, f)
        } else {
            f(self)
        }
    }
}

impl<B: ScopedBitRead> Reader for UperReader<B> {
    type Error = Error;

    #[inline]
    fn read<T: Readable>(&mut self) -> Result<T, Self::Error>
    where
        Self: Sized,
    {
        #[allow(clippy::let_and_return)]
        let value = T::read(self);
        #[cfg(feature = "descriptive-deserialize-errors")]
        let value = value.map_err(|mut e| {
            e.0.description = core::mem::take(&mut self.scope_description);
            e
        });
        value
    }

    #[inline]
    fn read_sequence<
        C: sequence::Constraint,
        S: Sized,
        F: Fn(&mut Self) -> Result<S, Self::Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<S, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::sequence::<C>());

        let _ = self.read_bit_field_entry(false);
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| {
            let extension_after = if let Some(extension_after) = C::EXTENDED_AFTER_FIELD {
                let bit_pos = r.bits.pos();
                if r.bits.read_bit()? {
                    Some((extension_after, bit_pos))
                } else {
                    None
                }
            } else {
                None
            };

            // In UPER the values for all OPTIONAL flags are written before any field
            // value is written. This remembers their position, so a later call of `read_opt`
            // can retrieve them from the buffer
            if r.bits.remaining() < C::STD_OPTIONAL_FIELDS as usize {
                return Err(ErrorKind::EndOfStream.into());
            }

            let range = r.bits.pos()..r.bits.pos() + C::STD_OPTIONAL_FIELDS as usize;
            r.bits.set_pos(range.end); // skip optional

            if let Some((extension_after, bit_pos)) = extension_after {
                r.scope_pushed(
                    Scope::ExtensibleSequence {
                        name: C::NAME,
                        bit_pos,
                        opt_bit_field: Some(range),
                        calls_until_ext_bitfield: (extension_after + 1) as usize,
                        number_of_ext_fields: (C::FIELD_COUNT - (extension_after + 1)) as usize,
                    },
                    f,
                )
            } else {
                r.scope_pushed(Scope::OptBitField(range), f)
            }
        });

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description.push(ScopeDescription::End(C::NAME));

        result
    }

    #[inline]
    fn read_sequence_of<C: sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<T::Type>, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::sequence_of::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        self.with_buffer(|r| {
            let len = if C::EXTENSIBLE {
                let extensible = r.bits.read_bit()?;
                if extensible {
                    r.read_length_determinant(None, None)?
                } else {
                    r.read_length_determinant(C::MIN, C::MAX)?
                }
            } else {
                r.read_length_determinant(C::MIN, C::MAX)?
            };

            if len > 0 {
                r.scope_stashed(|r| {
                    let mut vec = Vec::with_capacity(len as usize);
                    for _ in 0..len {
                        vec.push(T::read_value(r)?);
                    }
                    Ok(vec)
                })
            } else {
                Ok(Vec::new())
            }
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
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::enumerated::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| r.read_enumeration_index(C::STD_VARIANT_COUNT, C::EXTENSIBLE))
            .and_then(|index| {
                #[cfg(feature = "descriptive-deserialize-errors")]
                if index >= C::VARIANT_COUNT {
                    self.scope_description
                        .push(ScopeDescription::warning(format!(
                            "Index of extensible enum {} outside of known variants, clamping index value from {index} to {}",
                            C::NAME,
                            C::VARIANT_COUNT.saturating_sub(1)
                        )));
                }
                let result = C::from_choice_index(index)
                    .ok_or_else(|| ErrorKind::InvalidChoiceIndex(index, C::VARIANT_COUNT).into());
                #[cfg(feature = "descriptive-deserialize-errors")]
                self.scope_description.push(ScopeDescription::Result(
                    result.as_ref().map(|_| index.to_string()).map_err(Error::clone)
                ));
                result
            });

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description.push(ScopeDescription::End(C::NAME));

        result
    }

    #[inline]
    fn read_choice<C: choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description.push(ScopeDescription::choice::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.scope_stashed(|r| {
            let index = r.read_choice_index(C::STD_VARIANT_COUNT, C::EXTENSIBLE)?;
            let result = if index >= C::STD_VARIANT_COUNT {
                let length = r.read_length_determinant(None, None)?;
                r.read_whole_sub_slice(length as usize, |r| Ok((index, C::read_content(index, r)?)))
            } else {
                Ok((index, C::read_content(index, r)?))
            }
            .and_then(|(index, content)| {
                content.ok_or_else(|| ErrorKind::InvalidChoiceIndex(index, C::VARIANT_COUNT).into())
            });
            #[cfg(feature = "descriptive-deserialize-errors")]
            r.scope_description.push(ScopeDescription::Result(
                result
                    .as_ref()
                    .map(|_| index.to_string())
                    .map_err(Error::clone),
            ));
            result
        });

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description.push(ScopeDescription::End(C::NAME));

        result
    }

    #[inline]
    fn read_opt<T: ReadableType>(
        &mut self,
    ) -> Result<Option<<T as ReadableType>::Type>, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description.push(ScopeDescription::optional());

        // unwrap: as opt-field this must and will return some value
        if self.read_bit_field_entry(true)?.unwrap() {
            self.with_buffer(|w| w.scope_stashed(T::read_value))
                .map(Some)
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn read_default<C: default::Constraint<Owned = T::Type>, T: ReadableType>(
        &mut self,
    ) -> Result<T::Type, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::default_type());

        // unwrap: as opt-field this must and will return some value
        if self.read_bit_field_entry(true)?.unwrap() {
            self.scope_stashed(T::read_value)
        } else {
            Ok(C::DEFAULT_VALUE.to_owned())
        }
    }

    #[inline]
    #[allow(clippy::redundant_pattern_matching)] // allow for const_*!
    fn read_number<T: numbers::Number, C: numbers::Constraint<T>>(
        &mut self,
    ) -> Result<T, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::number::<T, C>());

        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|r| {
            let unconstrained = if C::EXTENSIBLE {
                r.bits.read_bit()?
            } else {
                const_is_none!(C::MIN) && const_is_none!(C::MAX)
            };

            let result = if unconstrained {
                r.bits.read_unconstrained_whole_number()
            } else {
                r.bits.read_constrained_whole_number(
                    const_unwrap_or!(C::MIN, 0),
                    const_unwrap_or!(C::MAX, i64::MAX),
                )
            };

            #[cfg(feature = "descriptive-deserialize-errors")]
            r.scope_description.push(ScopeDescription::Result(
                result
                    .as_ref()
                    .map(ToString::to_string)
                    .map_err(|e| e.clone()),
            ));

            result.map(T::from_i64)
        })
    }

    #[inline]
    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::utf8string::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| {
            // ITU-T X.691 | ISO/IEC 8825-2:2015, chapter 30.3
            // For 'known-multiplier character string types' there is no min/max in the encoding
            let octets = r.bits.read_octetstring(None, None, false)?;
            String::from_utf8(octets).map_err(|e| ErrorKind::FromUtf8Error(e).into())
        });

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::Result(result.clone()));

        result
    }

    #[inline]
    fn read_ia5string<C: ia5string::Constraint>(&mut self) -> Result<String, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::ia5string::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| {
            let len = if C::EXTENSIBLE && r.bits.read_bit()? {
                r.read_length_determinant(None, None)?
            } else {
                r.read_length_determinant(C::MIN, C::MAX)?
            };

            let mut buffer = vec![0u8; len as usize];
            for i in 0..len as usize {
                r.bits.read_bits_with_offset(&mut buffer[i..i + 1], 1)?;
            }

            String::from_utf8(buffer).map_err(|e| ErrorKind::FromUtf8Error(e).into())
        });

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::Result(result.clone()));

        result
    }

    #[inline]
    fn read_numeric_string<C: numericstring::Constraint>(&mut self) -> Result<String, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::numeric_string::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| {
            let len = if C::EXTENSIBLE && r.bits.read_bit()? {
                r.read_length_determinant(None, None)?
            } else {
                r.read_length_determinant(C::MIN, C::MAX)?
            };

            let mut buffer = vec![0u8; len as usize];
            for i in 0..len as usize {
                r.bits.read_bits_with_offset(&mut buffer[i..i + 1], 4)?;
                match buffer[i] {
                    0_u8 => buffer[i] = 32_u8,
                    c => buffer[i] = 32_u8 + 15 + c,
                }
            }

            String::from_utf8(buffer).map_err(|e| ErrorKind::FromUtf8Error(e).into())
        });

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::Result(result.clone()));

        result
    }

    #[inline]
    fn read_printable_string<C: printablestring::Constraint>(
        &mut self,
    ) -> Result<String, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::printable_string::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| {
            let len = if C::EXTENSIBLE && r.bits.read_bit()? {
                r.read_length_determinant(None, None)?
            } else {
                r.read_length_determinant(C::MIN, C::MAX)?
            };

            let mut buffer = vec![0u8; len as usize];
            buffer
                .chunks_exact_mut(1)
                .try_for_each(|chunk| r.bits.read_bits_with_offset(chunk, 1))?;

            String::from_utf8(buffer).map_err(|e| ErrorKind::FromUtf8Error(e).into())
        });

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::Result(result.clone()));

        result
    }

    #[inline]
    fn read_visible_string<C: visiblestring::Constraint>(&mut self) -> Result<String, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::visible_string::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| {
            let len = if C::EXTENSIBLE && r.bits.read_bit()? {
                r.read_length_determinant(None, None)?
            } else {
                r.read_length_determinant(C::MIN, C::MAX)?
            };

            let mut buffer = vec![0u8; len as usize];
            buffer
                .chunks_exact_mut(1)
                .try_for_each(|chunk| r.bits.read_bits_with_offset(chunk, 1))?;

            String::from_utf8(buffer).map_err(|e| ErrorKind::FromUtf8Error(e).into())
        });

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::Result(result.clone()));

        result
    }

    #[inline]
    fn read_octet_string<C: octetstring::Constraint>(&mut self) -> Result<Vec<u8>, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::octet_string::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| r.bits.read_octetstring(C::MIN, C::MAX, C::EXTENSIBLE));

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description.push(ScopeDescription::Result(
            result
                .as_ref()
                .map(|s| {
                    s.iter()
                        .map(|v| format!("{v:02x}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .map_err(|e| e.clone()),
        ));

        result
    }

    #[inline]
    fn read_bit_string<C: bitstring::Constraint>(&mut self) -> Result<(Vec<u8>, u64), Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::bit_string::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| r.bits.read_bitstring(C::MIN, C::MAX, C::EXTENSIBLE));

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description.push(ScopeDescription::Result(
            result
                .as_ref()
                .map(|(bits, len)| {
                    format!(
                        "len={len} bits=[{}]",
                        bits.iter()
                            .map(|v| format!("{v:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                })
                .map_err(|e| e.clone()),
        ));

        result
    }

    #[inline]
    fn read_boolean<C: boolean::Constraint>(&mut self) -> Result<bool, Self::Error> {
        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description
            .push(ScopeDescription::boolean::<C>());

        let _ = self.read_bit_field_entry(false)?;
        #[allow(clippy::let_and_return)]
        let result = self.with_buffer(|r| r.bits.read_boolean());

        #[cfg(feature = "descriptive-deserialize-errors")]
        self.scope_description.push(ScopeDescription::Result(
            result
                .as_ref()
                .map(|v| v.to_string())
                .map_err(|e| e.clone()),
        ));

        result
    }

    #[inline]
    fn read_null<C: null::Constraint>(&mut self) -> Result<Null, Self::Error> {
        Ok(Null)
    }
}

pub trait UperDecodable<'a, B: ScopedBitRead> {
    fn decode_from_uper(bits: B) -> Result<Self, Error>
    where
        Self: Sized;
}

impl<'a, R: Readable, B: ScopedBitRead> UperDecodable<'a, B> for R {
    fn decode_from_uper(bits: B) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let mut reader = UperReader::from(bits);
        Self::read(&mut reader)
    }
}

#[cfg(feature = "descriptive-deserialize-errors")]
#[cfg_attr(
    feature = "descriptive-deserialize-errors",
    derive(Debug, Clone, PartialEq)
)]
pub enum ScopeDescription {
    Root(Vec<ScopeDescription>),
    Sequence {
        tag: asn1rs_model::model::Tag,
        name: &'static str,
        std_optional_fields: u64,
        field_count: u64,
        extended_after_field: Option<u64>,
    },
    SequenceOf {
        tag: asn1rs_model::model::Tag,
        min: Option<u64>,
        max: Option<u64>,
        extensible: bool,
    },
    Enumerated {
        tag: asn1rs_model::model::Tag,
        name: &'static str,
        variant_count: u64,
        std_variant_count: u64,
        extensible: bool,
    },
    Choice {
        tag: asn1rs_model::model::Tag,
        name: &'static str,
        variant_count: u64,
        std_variant_count: u64,
        extensible: bool,
    },
    Optional,
    Default,
    Number {
        tag: asn1rs_model::model::Tag,
        min: Option<i64>,
        max: Option<i64>,
        extensible: bool,
    },
    Utf8String {
        tag: asn1rs_model::model::Tag,
        min: Option<u64>,
        max: Option<u64>,
        extensible: bool,
    },
    Ia5String {
        tag: asn1rs_model::model::Tag,
        min: Option<u64>,
        max: Option<u64>,
        extensible: bool,
    },
    NumericString {
        tag: asn1rs_model::model::Tag,
        min: Option<u64>,
        max: Option<u64>,
        extensible: bool,
    },
    PrintableString {
        tag: asn1rs_model::model::Tag,
        min: Option<u64>,
        max: Option<u64>,
        extensible: bool,
    },
    VisibleString {
        tag: asn1rs_model::model::Tag,
        min: Option<u64>,
        max: Option<u64>,
        extensible: bool,
    },
    OctetString {
        tag: asn1rs_model::model::Tag,
        min: Option<u64>,
        max: Option<u64>,
        extensible: bool,
    },
    BitString {
        tag: asn1rs_model::model::Tag,
        min: Option<u64>,
        max: Option<u64>,
        extensible: bool,
    },
    Boolean {
        tag: asn1rs_model::model::Tag,
    },
    Result(Result<String, Error>),
    BitsLengthDeterminant {
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
        result: Result<u64, Error>,
    },
    BitsEnumerationIndex {
        std_variants: u64,
        extensible: bool,
        result: Result<u64, Error>,
    },
    BitsChoiceIndex {
        std_variants: u64,
        extensible: bool,
        result: Result<u64, Error>,
    },
    ReadWholeSubSlice {
        length_bytes: usize,
        write_position: usize,
        write_original: usize,
        len: usize,
        result: Result<(), Error>,
    },
    ReadBitFieldEntry {
        is_opt: bool,
        result: Result<Option<bool>, Error>,
    },
    Warning {
        message: String,
    },
    Error {
        message: String,
    },
    End(&'static str),
}

#[cfg(feature = "descriptive-deserialize-errors")]
mod scope_description_impl {
    use super::*;

    impl ScopeDescription {
        #[inline]
        pub fn sequence<C: sequence::Constraint>() -> Self {
            Self::Sequence {
                tag: C::TAG,
                name: C::NAME,
                std_optional_fields: C::STD_OPTIONAL_FIELDS,
                field_count: C::FIELD_COUNT,
                extended_after_field: C::EXTENDED_AFTER_FIELD,
            }
        }

        #[inline]
        pub fn sequence_of<C: sequenceof::Constraint>() -> Self {
            Self::SequenceOf {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn enumerated<C: enumerated::Constraint>() -> Self {
            Self::Enumerated {
                tag: C::TAG,
                name: C::NAME,
                variant_count: C::VARIANT_COUNT,
                std_variant_count: C::STD_VARIANT_COUNT,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn choice<C: choice::Constraint>() -> Self {
            Self::Choice {
                tag: C::TAG,
                name: C::NAME,
                variant_count: C::VARIANT_COUNT,
                std_variant_count: C::STD_VARIANT_COUNT,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn optional() -> Self {
            ScopeDescription::Optional
        }

        #[inline]
        pub fn default_type() -> Self {
            ScopeDescription::Default
        }

        #[inline]
        pub fn number<T: numbers::Number, C: numbers::Constraint<T>>() -> Self {
            Self::Number {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn utf8string<C: utf8string::Constraint>() -> Self {
            Self::Utf8String {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn ia5string<C: ia5string::Constraint>() -> Self {
            Self::Ia5String {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn numeric_string<C: numericstring::Constraint>() -> Self {
            Self::NumericString {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn printable_string<C: printablestring::Constraint>() -> Self {
            Self::PrintableString {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn visible_string<C: visiblestring::Constraint>() -> Self {
            Self::VisibleString {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn octet_string<C: octetstring::Constraint>() -> Self {
            Self::OctetString {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn bit_string<C: bitstring::Constraint>() -> Self {
            Self::BitString {
                tag: C::TAG,
                min: C::MIN,
                max: C::MAX,
                extensible: C::EXTENSIBLE,
            }
        }

        #[inline]
        pub fn boolean<C: boolean::Constraint>() -> Self {
            Self::Boolean { tag: C::TAG }
        }

        #[inline]
        pub fn bits_length_determinant(
            lower_bound: Option<u64>,
            upper_bound: Option<u64>,
            result: Result<u64, Error>,
        ) -> Self {
            Self::BitsLengthDeterminant {
                lower_bound,
                upper_bound,
                result,
            }
        }

        #[inline]
        pub fn bits_enumeration_index(
            std_variants: u64,
            extensible: bool,
            result: Result<u64, Error>,
        ) -> Self {
            Self::BitsEnumerationIndex {
                std_variants,
                extensible,
                result,
            }
        }

        #[inline]
        pub fn bits_choice_index(
            std_variants: u64,
            extensible: bool,
            result: Result<u64, Error>,
        ) -> Self {
            Self::BitsChoiceIndex {
                std_variants,
                extensible,
                result,
            }
        }

        #[inline]
        pub fn read_whole_sub_slice<T>(
            length_bytes: usize,
            write_position: usize,
            write_original: usize,
            len: usize,
            result: &Result<T, Error>,
        ) -> Self {
            Self::ReadWholeSubSlice {
                length_bytes,
                write_position,
                write_original,
                len,
                result: result.as_ref().map(drop).map_err(|e| e.clone()),
            }
        }

        #[inline]
        pub fn read_bit_field_entry(is_opt: bool, result: &Result<Option<bool>, Error>) -> Self {
            Self::ReadBitFieldEntry {
                is_opt,
                result: result.clone(),
            }
        }

        #[inline]
        pub fn warning(s: impl Into<String>) -> Self {
            Self::Warning { message: s.into() }
        }
    }
}
