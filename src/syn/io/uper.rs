use crate::io::buffer::BitBuffer;
use crate::io::uper::Error as UperError;
use crate::io::uper::Reader as _UperReader;
use crate::io::uper::Writer as _UperWriter;
use crate::prelude::*;

pub struct ScopeStack<T> {
    scopes: Vec<Vec<T>>,
    scope: Vec<T>,
}

impl<T> Default for ScopeStack<T> {
    fn default() -> Self {
        ScopeStack {
            scopes: Vec::with_capacity(16),
            scope: Vec::default(),
        }
    }
}

impl<T> ScopeStack<T> {
    #[inline]
    pub fn current_mut(&mut self) -> &mut Vec<T> {
        &mut self.scope
    }

    #[inline]
    pub fn stash(&mut self) {
        self.push(Vec::default())
    }

    #[inline]
    pub fn push(&mut self, mut scope: Vec<T>) {
        std::mem::swap(&mut scope, &mut self.scope);
        self.scopes.push(scope);
    }

    #[inline]
    pub fn pop(&mut self) -> Vec<T> {
        let mut scope = self.scopes.pop().unwrap_or_default();
        std::mem::swap(&mut scope, &mut self.scope);
        scope
    }
}

#[derive(Default)]
pub struct UperWriter {
    buffer: BitBuffer,
    scope: ScopeStack<usize>,
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
    pub fn scope_pushed<R, F: Fn(&mut Self) -> R>(
        &mut self,
        scope: Vec<usize>,
        f: F,
    ) -> (R, Vec<usize>) {
        self.scope.push(scope);
        let result = f(self);
        (result, self.scope.pop())
    }

    #[inline]
    pub fn scope_stashed<R, F: Fn(&mut Self) -> R>(&mut self, f: F) -> R {
        self.scope.stash();
        let result = f(self);
        self.scope.pop();
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
        // In UPER the optional flag for all OPTIONAL values are written before any field
        // value is written. This reserves the bits, so that on a later call of `write_opt`
        // the value can be set to the actual state.
        let mut list = Vec::default();
        let write_pos = self.buffer.write_position;
        for i in (0..C::OPTIONAL_FIELDS).rev() {
            // insert in reverse order so that a simple pop() in `write_opt` retrieves
            // the relevant position
            list.push(write_pos + i);
            if let Err(e) = self.buffer.write_bit(false) {
                self.buffer.write_position = write_pos; // undo write_bits
                return Err(e);
            }
        }
        let (result, scope) = self.scope_pushed(list, f);
        result?; // first error on this before throwing non-informative assert errors
        debug_assert!(scope.is_empty());
        Ok(())
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
                w.buffer.write_choice_index_extensible(
                    choice.to_choice_index() as u64,
                    C::STD_VARIANT_COUNT as u64,
                )?;
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
        if let Some(position) = self.scope.current_mut().pop() {
            self.buffer
                .with_write_position_at(position, |buffer| buffer.write_bit(value.is_some()))?;
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
    scope: ScopeStack<bool>,
}

impl UperReader {
    pub fn from_bits<I: Into<Vec<u8>>>(bytes: I, bit_len: usize) -> Self {
        Self {
            buffer: BitBuffer::from_bits(bytes.into(), bit_len),
            scope: Default::default(),
        }
    }

    pub fn bits_remaining(&self) -> usize {
        self.buffer.write_position - self.buffer.read_position
    }

    #[inline]
    pub fn scope_pushed<R, F: Fn(&mut Self) -> R>(
        &mut self,
        scope: Vec<bool>,
        f: F,
    ) -> (R, Vec<bool>) {
        self.scope.push(scope);
        let result = f(self);
        (result, self.scope.pop())
    }

    #[inline]
    pub fn scope_stashed<R, F: Fn(&mut Self) -> R>(&mut self, f: F) -> R {
        self.scope.stash();
        let result = f(self);
        self.scope.pop();
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
        // In UPER the optional flag for all OPTIONAL values are written before any field
        // value is written. This loads those bits, so that on a later call of `read_opt` can
        // retrieve them by a simple call of `pop` on the optionals buffer
        let mut optionals = vec![false; C::OPTIONAL_FIELDS];
        for i in (0..C::OPTIONAL_FIELDS).rev() {
            optionals[i] = self.buffer.read_bit()?;
        }
        let (result, scope) = self.scope_pushed(optionals, f);
        let result = result?; // first error on this before throwing non-informative assert errors
        debug_assert!(scope.is_empty());
        Ok(result)
    }

    #[inline]
    fn read_sequence_of<C: sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<T::Type>, Self::Error> {
        let min = C::MIN.unwrap_or(0);
        let max = C::MAX.unwrap_or(std::usize::MAX);
        let len = self.buffer.read_length_determinant()? + min; // TODO untested for MIN != 0
        if len > max {
            return Err(UperError::SizeNotInRange(len, min, max));
        }
        self.scope_stashed(|w| {
            let mut vec = Vec::with_capacity(len);
            for _ in 0..len {
                vec.push(T::read_value(w)?);
            }
            Ok(vec)
        })
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
        self.scope_stashed(|w| {
            if C::EXTENSIBLE {
                w.buffer
                    .read_choice_index_extensible(C::STD_VARIANT_COUNT as u64)
                    .map(|v| v as usize)
            } else {
                w.buffer
                    .read_choice_index(C::STD_VARIANT_COUNT as u64)
                    .map(|v| v as usize)
            }
            .and_then(|index| {
                C::read_content(index, w)?
                    .ok_or_else(|| UperError::InvalidChoiceIndex(index, C::VARIANT_COUNT))
            })
        })
    }

    #[inline]
    fn read_opt<T: ReadableType>(
        &mut self,
    ) -> Result<Option<<T as ReadableType>::Type>, Self::Error> {
        let value = if let Some(pre_fetched) = self.scope.current_mut().pop() {
            pre_fetched
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
