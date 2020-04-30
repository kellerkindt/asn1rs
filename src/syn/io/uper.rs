use crate::io::buffer::BitBuffer;
use crate::io::uper::Error as UperError;
use crate::io::uper::Reader as _UperReader;
use crate::io::uper::Writer as _UperWriter;
use crate::prelude::*;

#[derive(Default)]
pub struct UperWriter {
    buffer: BitBuffer,
    optional_positions: Vec<usize>,
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
}

impl Writer for UperWriter {
    type Error = UperError;

    fn write_sequence<C: sequence::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error> {
        // In UPER the optional flag for all OPTIONAL values are written before any field
        // value is written. This reserves the bits, so that on a later call of `write_opt`
        // the value can be set to the actual state.
        let before = self.optional_positions.len();
        let write_pos = self.buffer.write_position;
        for i in (0..C::OPTIONAL_FIELDS).rev() {
            // insert in reverse order so that a simple pop() in `write_opt` retrieves
            // the relevant position
            self.optional_positions.push(write_pos + i);
            if let Err(e) = self.buffer.write_bit(false) {
                self.buffer.write_position = write_pos; // undo write_bits
                return Err(e);
            }
        }
        f(self)?;
        assert_eq!(before, self.optional_positions.len());
        Ok(())
    }

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
            self.buffer.write_int(
                enumerated.to_choice_index() as i64,
                (0, C::STD_VARIANT_COUNT as i64),
            )
        }
    }

    fn write_opt<T: WritableType>(
        &mut self,
        value: Option<&<T as WritableType>::Type>,
    ) -> Result<(), Self::Error> {
        self.buffer
            .with_write_position_at(self.optional_positions.pop().unwrap(), |buffer| {
                buffer.write_bit(value.is_some())
            })?;
        if let Some(value) = value {
            T::write_value(self, value)
        } else {
            Ok(())
        }
    }

    fn write_int(&mut self, value: i64, range: (i64, i64)) -> Result<(), Self::Error> {
        self.buffer.write_int(value, range)
    }

    fn write_int_max(&mut self, value: u64) -> Result<(), Self::Error> {
        self.buffer.write_int_max(value)
    }

    fn write_utf8string<C: utf8string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        self.buffer.write_utf8_string(value)
    }
}

pub struct UperReader {
    buffer: BitBuffer,
    optionals: Vec<bool>,
}

impl UperReader {
    pub fn from_bits<I: Into<Vec<u8>>>(bytes: I, bit_len: usize) -> Self {
        Self {
            buffer: BitBuffer::from_bits(bytes.into(), bit_len),
            optionals: Vec::default(),
        }
    }

    pub fn bits_remaining(&self) -> usize {
        self.buffer.write_position - self.buffer.read_position
    }
}

impl Reader for UperReader {
    type Error = UperError;

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
        let position = self.optionals.len();
        self.optionals.resize(position + C::OPTIONAL_FIELDS, false);
        for i in (0..C::OPTIONAL_FIELDS).rev() {
            self.optionals[position + i] = match self.buffer.read_bit() {
                Ok(bit) => bit,
                Err(e) => {
                    // need to remove eagerly added values
                    self.optionals.resize(position, false);
                    return Err(e);
                }
            }
        }
        let result = f(self);
        assert_eq!(position, self.optionals.len());
        result
    }

    fn read_enumerated<C: enumerated::Constraint>(&mut self) -> Result<C, Self::Error> {
        if C::EXTENSIBLE {
            self.buffer
                .read_choice_index_extensible(C::STD_VARIANT_COUNT as u64)
                .map(|v| v as usize)
        } else {
            self.read_int((0, C::STD_VARIANT_COUNT as i64))
                .map(|v| v as usize)
        }
        .and_then(|index| {
            C::from_choice_index(index)
                .ok_or_else(|| UperError::InvalidChoiceIndex(index, C::VARIANT_COUNT))
        })
    }

    fn read_opt<T: ReadableType>(
        &mut self,
    ) -> Result<Option<<T as ReadableType>::Type>, Self::Error> {
        if self.optionals.pop().unwrap() {
            T::read_value(self).map(Some)
        } else {
            Ok(None)
        }
    }

    fn read_int(&mut self, range: (i64, i64)) -> Result<i64, Self::Error> {
        self.buffer.read_int(range)
    }

    fn read_int_max(&mut self) -> Result<u64, Self::Error> {
        self.buffer.read_int_max()
    }

    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error> {
        self.buffer.read_utf8_string()
    }
}
