use byteorder::ByteOrder;
use byteorder::NetworkEndian;

use io::uper::Error as UperError;
use io::uper::Reader as UperReader;
use io::uper::Writer as UperWriter;
use io::uper::BYTE_LEN;
use io::CodecReader;
use io::CodecWriter;

#[derive(Debug, Default)]
pub struct BitBuffer {
    buffer: Vec<u8>,
    write_position: usize,
    read_position: usize,
}

impl BitBuffer {
    #[allow(unused)]
    pub fn from(buffer: Vec<u8>, bit_length: usize) -> BitBuffer {
        assert!(bit_length <= buffer.len() * 8);
        BitBuffer {
            buffer,
            write_position: bit_length,
            read_position: 0,
        }
    }

    #[allow(unused)]
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.write_position = 0;
        self.read_position = 0;
    }

    #[allow(unused)]
    pub fn reset_read_position(&mut self) {
        self.read_position = 0;
    }

    #[allow(unused)]
    pub fn content(&self) -> &[u8] {
        &self.buffer[..]
    }

    #[allow(unused)]
    pub fn bit_len(&self) -> usize {
        self.write_position
    }

    #[allow(unused)]
    pub fn byte_len(&self) -> usize {
        self.buffer.len()
    }
}

impl CodecReader for BitBuffer {}
impl UperReader for BitBuffer {
    fn read_utf8_string(&mut self) -> Result<String, UperError> {
        let len = self.read_length_determinant()?;
        let mut buffer = vec![0u8; len];
        self.read_bit_string_till_end(&mut buffer[..len], 0)?;
        if let Ok(string) = ::std::str::from_utf8(&buffer[..len]) {
            Ok(string.into())
        } else {
            Err(UperError::InvalidUtf8String)
        }
    }

    fn read_int(&mut self, range: (i64, i64)) -> Result<i64, UperError> {
        let (lower, upper) = range;
        let range = (upper - lower) as u64;
        let bit_length_range = {
            let mut range = range;
            let mut bit_length: u8 = 0;
            while range > 0 {
                bit_length += 1;
                range /= 2;
            }
            bit_length
        };

        let mut buffer = [0u8; 8];
        let buffer_bits = buffer.len() * BYTE_LEN as usize;
        debug_assert!(buffer_bits == 64);
        self.read_bit_string_till_end(&mut buffer[..], buffer_bits - bit_length_range as usize)?;
        let value = NetworkEndian::read_u64(&buffer[..]) as i64;
        Ok(value + lower)
    }

    fn read_bit_string(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), UperError> {
        if buffer.len() * BYTE_LEN < bit_offset || buffer.len() * BYTE_LEN < bit_offset + bit_length
        {
            return Err(UperError::InsufficientSpaceInDestinationBuffer);
        }
        for bit in bit_offset..bit_offset + bit_length {
            let byte_pos = bit / BYTE_LEN;
            let bit_pos = bit % BYTE_LEN;
            let bit_pos = BYTE_LEN - bit_pos - 1; // flip

            if self.read_bit()? {
                // set bit
                buffer[byte_pos] |= 0x01 << bit_pos;
            } else {
                // reset bit
                buffer[byte_pos] &= !(0x01 << bit_pos);
            }
        }
        Ok(())
    }

    fn read_length_determinant(&mut self) -> Result<usize, UperError> {
        if !self.read_bit()? {
            // length < ::std::i8::MAX as usize
            Ok(self.read_int((0, ::std::i8::MAX as i64 - 1))? as usize)
        } else if self.read_bit()? {
            // length < ::std::i16::MAX as usize
            Ok(self.read_int((0, ::std::i16::MAX as i64 - 1))? as usize)
        } else {
            Err(UperError::UnsupportedOperation(
                "Cannot read length determinant for other than i8 and i16".into(),
            ))
        }
    }

    fn read_bit(&mut self) -> Result<bool, UperError> {
        if self.read_position >= self.write_position {
            return Err(UperError::EndOfStream);
        }
        let byte_pos = self.read_position as usize / BYTE_LEN;
        let bit_pos = self.read_position % BYTE_LEN;
        let bit_pos = (BYTE_LEN - bit_pos - 1) as u8; // flip
        let mask = 0x01 << bit_pos;
        let bit = (self.buffer[byte_pos] & mask) == mask;
        self.read_position += 1;
        Ok(bit)
    }
}

impl CodecWriter for BitBuffer {}
impl UperWriter for BitBuffer {
    fn write_utf8_string(&mut self, value: &str) -> Result<(), UperError> {
        self.write_length_determinant(value.len())?;
        self.write_bit_string_till_end(value.as_bytes(), 0)?;
        Ok(())
    }

    fn write_int(&mut self, value: i64, range: (i64, i64)) -> Result<(), UperError> {
        let (lower, upper) = range;
        let value = {
            if value > upper || value < lower {
                return Err(UperError::ValueNotInRange(value, lower, upper));
            }
            (value - lower) as u64
        };
        let range = (upper - lower) as u64;
        let bit_length_range = {
            let mut range = range;
            let mut bit_length: u8 = 0;
            while range > 0 {
                bit_length += 1;
                range /= 2;
            }
            bit_length
        };

        let mut buffer = [0u8; 8];
        NetworkEndian::write_u64(&mut buffer[..], value);
        let buffer_bits = buffer.len() * BYTE_LEN as usize;
        debug_assert!(buffer_bits == 64);

        self.write_bit_string_till_end(&buffer[..], buffer_bits - bit_length_range as usize)?;

        Ok(())
    }

    fn write_bit_string(
        &mut self,
        buffer: &[u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), UperError> {
        if buffer.len() * BYTE_LEN < bit_offset || buffer.len() * BYTE_LEN < bit_offset + bit_length
        {
            return Err(UperError::InsufficientDataInSourceBuffer);
        }
        for bit in bit_offset..bit_offset + bit_length {
            let byte_pos = bit / BYTE_LEN;
            let bit_pos = bit % BYTE_LEN;
            let bit_pos = BYTE_LEN - bit_pos - 1; // flip

            let bit = (buffer[byte_pos] >> bit_pos & 0x01) == 0x01;
            self.write_bit(bit)?;
        }
        Ok(())
    }

    fn write_length_determinant(&mut self, length: usize) -> Result<(), UperError> {
        if length < ::std::i8::MAX as usize {
            self.write_bit(false)?;
            self.write_int(length as i64, (0, ::std::i8::MAX as i64 - 1))
        } else if length < ::std::i16::MAX as usize {
            self.write_bit(true)?;
            self.write_bit(false)?;
            self.write_int(length as i64, (0, ::std::i16::MAX as i64 - 1))
        } else {
            Err(UperError::UnsupportedOperation(format!(
                "Writing length determinant for lengths > {} is unsupported, tried for length {}",
                ::std::i16::MAX,
                length
            )))
        }
    }

    fn write_bit(&mut self, bit: bool) -> Result<(), UperError> {
        let byte_pos = self.write_position as usize / BYTE_LEN;
        let bit_pos = self.write_position % BYTE_LEN;
        if bit_pos == 0 {
            self.buffer.push(0x00);
        }
        if bit {
            let bit_pos = (BYTE_LEN - bit_pos - 1) as u8; // flip
            self.buffer[byte_pos] |= 0x01 << bit_pos;
        }
        self.write_position += 1;
        Ok(())
    }
}
