#![allow(clippy::unusual_byte_groupings)]

use crate::protocol::basic::err::Error;
use crate::protocol::basic::{BasicRead, BasicWrite};
use crate::rw::{BasicReader, BasicWriter};
use asn1rs_model::asn::Tag;
use std::io::{Read, Write};

pub type DER = DistinguishedEncodingRules;
pub struct DistinguishedEncodingRules;

impl DistinguishedEncodingRules {
    #[inline]
    pub fn writer<W: BasicWrite<Flavor = Self>>(write: W) -> BasicWriter<W> {
        BasicWriter::from(write)
    }

    #[inline]
    pub fn reader<W: BasicRead<Flavor = Self>>(read: W) -> BasicReader<W> {
        BasicReader::from(read)
    }
}

const CLASS_BITS_MASK: u8 = 0b_11_000000;
const CLASS_BITS_UNIVERSAL: u8 = 0b_00_000000;
const CLASS_BITS_APPLICATION: u8 = 0b_01_000000;
const CLASS_BITS_CONTEXT_SPECIFIC: u8 = 0b_10_000000;
const CLASS_BITS_PRIVATE: u8 = 0b_11_000000;

const LENGTH_SHORT_MAX_VALUE: u64 = 127;
const LENGTH_BIT_MASK: u8 = 0b1_0000000;
const LENGTH_BIT_SHORT_FORM: u8 = 0b0_0000000;
const LENGTH_BIT_LONG_FORM: u8 = 0b1_0000000;

impl<T: Read> BasicRead for T {
    type Flavor = DistinguishedEncodingRules;

    fn read_identifier(&mut self) -> Result<Tag, Error> {
        let mut byte = [0x00];
        self.read_exact(&mut byte[..])?;
        let class = byte[0] & CLASS_BITS_MASK;
        let value = byte[0] & !CLASS_BITS_MASK;
        // TODO assumption: number contains the primitive / constructed flag
        // TODO assumption: number not greater than the octets remaining bits
        Ok(match class {
            CLASS_BITS_UNIVERSAL => Tag::Universal(usize::from(value)),
            CLASS_BITS_APPLICATION => Tag::Application(usize::from(value)),
            CLASS_BITS_CONTEXT_SPECIFIC => Tag::ContextSpecific(usize::from(value)),
            CLASS_BITS_PRIVATE => Tag::Private(usize::from(value)),
            _ => unreachable!(),
        })
    }

    #[inline]
    fn read_length(&mut self) -> Result<u64, Error> {
        let mut bytes = [0u8; 1];
        self.read_exact(&mut bytes[..])?;
        if bytes[0] & LENGTH_BIT_MASK == LENGTH_BIT_SHORT_FORM {
            Ok(u64::from(bytes[0] & !LENGTH_BIT_MASK))
        } else {
            let byte_length = (bytes[0] & !LENGTH_BIT_MASK) as u32;
            self.read_integer_u64(byte_length)
        }
    }

    #[inline]
    fn read_boolean(&mut self) -> Result<bool, Error> {
        let mut byte = [0u8; 1];
        self.read_exact(&mut byte[..])?;
        Ok(byte[0] != 0x00)
    }

    fn read_integer_i64(&mut self, byte_len: u32) -> Result<i64, Error> {
        let mut bytes = 0i64.to_be_bytes();

        if byte_len as usize > bytes.len() {
            return Err(Error::unsupported_byte_len(
                bytes.len() as u8,
                byte_len as u8,
            ));
        }

        let offset = bytes.len() - byte_len as usize;
        self.read_exact(&mut bytes[offset..])?;
        Ok(i64::from_be_bytes(bytes))
    }

    fn read_integer_u64(&mut self, byte_len: u32) -> Result<u64, Error> {
        let mut bytes = 0u64.to_be_bytes();

        if byte_len as usize > bytes.len() {
            return Err(Error::unsupported_byte_len(
                bytes.len() as u8,
                byte_len as u8,
            ));
        }

        let offset = bytes.len() - byte_len as usize;
        self.read_exact(&mut bytes[offset..])?;
        Ok(u64::from_be_bytes(bytes))
    }
}

impl<T: Write> BasicWrite for T {
    type Flavor = DistinguishedEncodingRules;

    fn write_identifier(&mut self, tag: Tag) -> Result<(), Error> {
        let mut identifier_octet: u8 = match tag {
            Tag::Universal(_) => CLASS_BITS_UNIVERSAL,
            Tag::Application(_) => CLASS_BITS_APPLICATION,
            Tag::ContextSpecific(_) => CLASS_BITS_CONTEXT_SPECIFIC,
            Tag::Private(_) => CLASS_BITS_PRIVATE,
        };
        // TODO assumption: number contains the primitive / constructed flag
        // TODO assumption: number not greater than the octets remaining bits
        identifier_octet |= tag.value() as u8;
        Ok(self.write_all(&[identifier_octet])?)
    }

    #[inline]
    fn write_length(&mut self, length: u64) -> Result<(), Error> {
        if length <= LENGTH_SHORT_MAX_VALUE {
            // short form 8.1.3.4
            let byte = LENGTH_BIT_SHORT_FORM | (length as u8);
            Ok(self.write_all(&[byte])?)
        } else {
            // long form 8.1.3.5
            let leading_zero_bits = length.leading_zeros();
            let leading_zero_bytes = leading_zero_bits / u8::BITS;
            let len_bytes = length.to_be_bytes().len() as u32 - leading_zero_bytes;

            self.write_all(&[LENGTH_BIT_LONG_FORM | len_bytes as u8])?;
            self.write_integer_u64(length)
        }
    }

    #[inline]
    fn write_boolean(&mut self, value: bool) -> Result<(), Error> {
        Ok(self.write_all(&[if value { 0x01 } else { 0x00 }])?)
    }

    #[inline]
    fn write_integer_i64(&mut self, value: i64) -> Result<(), Error> {
        let bytes = value.to_be_bytes();
        let offset = (value.leading_zeros() / u8::BITS).min(bytes.len() as u32 - 1);
        self.write_all(&bytes[offset as usize..])?;
        Ok(())
    }

    #[inline]
    fn write_integer_u64(&mut self, value: u64) -> Result<(), Error> {
        let bytes = value.to_be_bytes();
        let offset = (value.leading_zeros() / u8::BITS).min(bytes.len() as u32 - 1);
        self.write_all(&bytes[offset as usize..])?;
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn write_read_length_check(len: u64) {
        let mut buffer = Vec::new();
        buffer.write_length(len).unwrap();
        assert_eq!(len, (&mut &buffer[..]).read_length().unwrap());
    }

    #[test]
    pub fn test_length_bounds() {
        write_read_length_check(0);
        write_read_length_check(LENGTH_SHORT_MAX_VALUE - 1);
        write_read_length_check(LENGTH_SHORT_MAX_VALUE);
        write_read_length_check(LENGTH_SHORT_MAX_VALUE + 1);
        write_read_length_check(u8::MAX as u64 - 1);
        write_read_length_check(u8::MAX as u64);
        write_read_length_check(u8::MAX as u64 + 1);
        write_read_length_check(u16::MAX as u64 - 1);
        write_read_length_check(u16::MAX as u64);
        write_read_length_check(u16::MAX as u64 + 1);
        write_read_length_check(u32::MAX as u64 - 1);
        write_read_length_check(u32::MAX as u64);
        write_read_length_check(u32::MAX as u64 + 1);
        write_read_length_check(u64::MAX - 1);
        write_read_length_check(u64::MAX);
    }
}
