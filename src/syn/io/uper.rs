use crate::io::buffer::BitBuffer;
use crate::io::uper::Error as UperError;
use crate::io::uper::Reader as _UperReader;
use crate::io::uper::Writer as _UperWriter;
use crate::prelude::*;
use std::ops::Range;

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
    pub fn exhausted(&self) -> bool {
        match self {
            Scope::OptBitField(range) => range.start == range.end,
            Scope::AllBitField(range) => range.start == range.end,
            Scope::ExtensibleSequence { .. } => false,
        }
    }

    pub fn encode_as_open_type_field(&self) -> bool {
        matches!(self, Scope::AllBitField(_))
    }

    pub fn write_into_field(
        &mut self,
        buffer: &mut BitBuffer,
        is_opt: bool,
        is_present: bool,
    ) -> Result<(), UperError> {
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
                    buffer.write_int_normally_small(*number_of_ext_fields as u64 - 1)?;
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

    pub fn read_from_field(
        &mut self,
        buffer: &mut BitBuffer,
        is_opt: bool,
    ) -> Result<Option<bool>, UperError> {
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
                    let read_number_of_ext_fields = buffer.read_int_normally_small()? as usize + 1;
                    if read_number_of_ext_fields != *number_of_ext_fields {
                        return Err(UperError::UnsupportedOperation(format!(
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
    pub fn write_bit_field_entry(
        &mut self,
        is_opt: bool,
        is_present: bool,
    ) -> Result<(), UperError> {
        if let Some(scope) = &mut self.scope {
            scope.write_into_field(&mut self.buffer, is_opt, is_present)
        } else if is_opt {
            self.buffer.write_bit(is_present)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn with_buffer<T, F: FnOnce(&mut Self) -> Result<T, UperError>>(
        &mut self,
        f: F,
    ) -> Result<T, UperError> {
        if self
            .scope
            .as_ref()
            .map(Scope::encode_as_open_type_field)
            .unwrap_or(false)
        {
            let mut writer = UperWriter::default();
            let result = f(&mut writer)?;
            self.buffer
                .write_length_determinant(writer.buffer.byte_len())?;
            self.buffer
                .write_bit_string_till_end(writer.buffer.content(), 0)?;
            Ok(result)
        } else {
            f(self)
        }
    }
}

impl Writer for UperWriter {
    type Error = UperError;

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
            let range = write_pos..write_pos + C::STD_OPTIONAL_FIELDS; // TODO
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
                        calls_until_ext_bitfield: extension_after + 1,
                        number_of_ext_fields: C::FIELD_COUNT - (extension_after + 1),
                    },
                    f,
                )
            } else {
                w.scope_pushed(Scope::OptBitField(range), f)
            }
        })
    }

    #[inline]
    fn write_sequence_of<C: sequenceof::Constraint, T: WritableType>(
        &mut self,
        slice: &[T::Type],
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
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
        self.write_bit_field_entry(false, true)?;
        if C::EXTENSIBLE {
            self.with_buffer(|w| {
                w.buffer.write_choice_index_extensible(
                    enumerated.to_choice_index() as u64,
                    C::STD_VARIANT_COUNT as u64,
                )
            })
        } else {
            self.with_buffer(|w| {
                w.buffer.write_choice_index(
                    enumerated.to_choice_index() as u64,
                    C::STD_VARIANT_COUNT as u64,
                )
            })
        }
    }

    #[inline]
    fn write_choice<C: choice::Constraint>(&mut self, choice: &C) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
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
        self.write_bit_field_entry(true, value.is_some())?;
        if let Some(value) = value {
            self.scope_stashed(|w| T::write_value(w, value))
        } else {
            Ok(())
        }
    }

    #[inline]
    fn write_int(&mut self, value: i64, range: (i64, i64)) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| w.buffer.write_int(value, range))
    }

    #[inline]
    fn write_int_max(&mut self, value: u64) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| w.buffer.write_int_max(value))
    }

    #[inline]
    fn write_utf8string<C: utf8string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| w.buffer.write_utf8_string(value))
    }

    #[inline]
    fn write_octet_string<C: octetstring::Constraint>(
        &mut self,
        value: &[u8],
    ) -> Result<(), Self::Error> {
        self.write_bit_field_entry(false, true)?;
        self.with_buffer(|w| w.buffer.write_octet_string(value, bit_buffer_range::<C>()))
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
    pub fn read_bit_field_entry(&mut self, is_opt: bool) -> Result<Option<bool>, UperError> {
        if let Some(scope) = &mut self.scope {
            scope.read_from_field(&mut self.buffer, is_opt)
        } else if is_opt {
            Some(self.buffer.read_bit()).transpose()
        } else {
            Ok(None)
        }
    }

    #[inline]
    pub fn with_buffer<T, F: FnOnce(&mut Self) -> Result<T, UperError>>(
        &mut self,
        f: F,
    ) -> Result<T, UperError> {
        if self
            .scope
            .as_ref()
            .map(Scope::encode_as_open_type_field)
            .unwrap_or(false)
        {
            let len = self.buffer.read_length_determinant()?;
            self.read_whole_sub_slice(len, f)
        } else {
            f(self)
        }
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
        let _ = self.read_bit_field_entry(false);
        self.with_buffer(|w| {
            if let Some(extension_after) = C::EXTENDED_AFTER_FIELD {
                let has_extension = w.buffer.read_bit()?;
                let expects_extension = C::FIELD_COUNT > extension_after;
                if has_extension != expects_extension {
                    return Err(UperError::InvalidExtensionConstellation(
                        expects_extension,
                        has_extension,
                    ));
                }
            }

            // In UPER the values for all OPTIONAL flags are written before any field
            // value is written. This remembers their position, so a later call of `read_opt`
            // can retrieve them from the buffer
            let range = w.buffer.read_position..w.buffer.read_position + C::STD_OPTIONAL_FIELDS;
            if w.buffer.bit_len() < range.end {
                return Err(UperError::EndOfStream);
            }
            w.buffer.read_position = range.end; // skip optional

            if let Some(extension_after) = C::EXTENDED_AFTER_FIELD {
                w.scope_pushed(
                    Scope::ExtensibleSequence {
                        opt_bit_field: Some(range),
                        calls_until_ext_bitfield: extension_after + 1,
                        number_of_ext_fields: C::FIELD_COUNT - (extension_after + 1),
                    },
                    f,
                )
            } else {
                w.scope_pushed(Scope::OptBitField(range), f)
            }
        })
    }

    #[inline]
    fn read_sequence_of<C: sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<T::Type>, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|w| {
            let min = C::MIN.unwrap_or(0);
            let max = C::MAX.unwrap_or(std::usize::MAX);
            let len = w.buffer.read_length_determinant()? + min; // TODO untested for MIN != 0
            if len > max {
                Err(UperError::SizeNotInRange(len, min, max))
            } else {
                w.scope_stashed(|w| {
                    let mut vec = Vec::with_capacity(len);
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
        if C::EXTENSIBLE {
            self.with_buffer(|w| {
                w.buffer
                    .read_choice_index_extensible(C::STD_VARIANT_COUNT as u64)
                    .map(|v| v as usize)
            })
        } else {
            self.with_buffer(|w| {
                w.buffer
                    .read_choice_index(C::STD_VARIANT_COUNT as u64)
                    .map(|v| v as usize)
            })
        }
        .and_then(|index| {
            C::from_choice_index(index)
                .ok_or_else(|| UperError::InvalidChoiceIndex(index, C::VARIANT_COUNT))
        })
    }

    #[inline]
    fn read_choice<C: choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
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
        // unwrap: as opt-field this must and will return some value
        if self.read_bit_field_entry(true)?.unwrap() {
            self.scope_stashed(T::read_value).map(Some)
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn read_int(&mut self, range: (i64, i64)) -> Result<i64, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|w| w.buffer.read_int(range))
    }

    #[inline]
    fn read_int_max(&mut self) -> Result<u64, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|w| w.buffer.read_int_max())
    }

    #[inline]
    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|w| w.buffer.read_utf8_string())
    }

    #[inline]
    fn read_octet_string<C: octetstring::Constraint>(&mut self) -> Result<Vec<u8>, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|w| w.buffer.read_octet_string(bit_buffer_range::<C>()))
    }

    #[inline]
    fn read_boolean<C: boolean::Constraint>(&mut self) -> Result<bool, Self::Error> {
        let _ = self.read_bit_field_entry(false)?;
        self.with_buffer(|w| w.buffer.read_bit())
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
