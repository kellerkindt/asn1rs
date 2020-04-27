use crate::io::buffer::BitBuffer;
use crate::io::uper::Error as UperError;
use crate::io::uper::Writer as _;
use crate::prelude::*;
use std::fmt::{Display, Formatter};

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
}

impl Writer for UperWriter {
    type Error = UperError;

    fn write_sequence<C: sequence::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error> {
        // in UPER the optional flag for all OPTIONAL values is written before any field
        // value is written. This reserves the bits, so that on a later call of `write_opt`
        // the value can be set to the actual state.
        let before = self.optional_positions.len();
        let write_pos = self.buffer.write_position;
        for i in (0..C::OPTIONAL_FIELDS).rev() {
            // insert in reverse order so that a simple pop() in `write_opt` retrieves
            // the relevant position
            self.optional_positions.push(write_pos + i);
            self.buffer.write_bit(false);
        }
        f(self)?;
        assert_eq!(before, self.optional_positions.len());
        Ok(())
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
