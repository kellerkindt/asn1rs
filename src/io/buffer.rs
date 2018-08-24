use byteorder::ByteOrder;
use byteorder::NetworkEndian;

use io::uper::Error as UperError;
use io::uper::Reader as UperReader;
use io::uper::Writer as UperWriter;
use io::uper::BYTE_LEN;

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

const UPER_LENGTH_DET_L1: i64 = 127;
const UPER_LENGTH_DET_L2: i64 = 16383;
// const UPER_LENGTH_DET_L3: i64 = 49151;
// const UPER_LENGTH_DET_L4: i64 = 65535;

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

    fn read_int_max(&mut self) -> Result<u64, UperError> {
        let len_in_bytes = self.read_length_determinant()?;
        if len_in_bytes > 8 {
            Err(UperError::UnsupportedOperation(
                "Reading bigger data types than 64bit is not supported".into(),
            ))
        } else {
            let mut buffer = vec![0u8; 8];
            let offset = (8 * BYTE_LEN) - (len_in_bytes * BYTE_LEN);
            self.read_bit_string_till_end(&mut buffer[..], offset)?;
            Ok(NetworkEndian::read_u64(&buffer[..]))
        }
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
            // length <= UPER_LENGTH_DET_L1
            Ok(self.read_int((0, UPER_LENGTH_DET_L1))? as usize)
        } else if !self.read_bit()? {
            // length <= UPER_LENGTH_DET_L2
            Ok(self.read_int((0, UPER_LENGTH_DET_L2))? as usize)
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

    fn write_int_max(&mut self, value: u64) -> Result<(), UperError> {
        if value > ::std::i64::MAX as u64 {
            return Err(UperError::ValueNotInRange(value as i64, 0, ::std::i64::MAX));
        }
        let mut buffer = [0u8; 8];
        NetworkEndian::write_u64(&mut buffer[..], value);
        let byte_len = {
            let mut len = buffer.len();
            while len > 0 && buffer[buffer.len() - len] == 0x00 {
                len -= 1;
            }
            len
        };
        self.write_length_determinant(byte_len)?;
        if byte_len > 0 {
            let bit_offset = (buffer.len() - byte_len) * BYTE_LEN;
            self.write_bit_string_till_end(&buffer, bit_offset)?;
        }

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
        if length <= UPER_LENGTH_DET_L1 as usize {
            self.write_bit(false)?;
            self.write_int(length as i64, (0, UPER_LENGTH_DET_L1))
        } else if length <= UPER_LENGTH_DET_L2 as usize {
            self.write_bit(true)?;
            self.write_bit(false)?;
            self.write_int(length as i64, (0, UPER_LENGTH_DET_L2))
        } else {
            Err(UperError::UnsupportedOperation(format!(
                "Writing length determinant for lengths > {} is unsupported, tried for length {}",
                UPER_LENGTH_DET_L2, length
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn bit_buffer_write_bit_keeps_correct_order() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();

        buffer.write_bit(true)?;
        buffer.write_bit(false)?;
        buffer.write_bit(false)?;
        buffer.write_bit(true)?;

        buffer.write_bit(true)?;
        buffer.write_bit(true)?;
        buffer.write_bit(true)?;
        buffer.write_bit(false)?;

        assert_eq!(buffer.content(), &[0b1001_1110]);

        buffer.write_bit(true)?;
        buffer.write_bit(false)?;
        buffer.write_bit(true)?;
        buffer.write_bit(true)?;

        buffer.write_bit(true)?;
        buffer.write_bit(true)?;
        buffer.write_bit(true)?;
        buffer.write_bit(false)?;

        assert_eq!(buffer.content(), &[0b1001_1110, 0b1011_1110]);

        buffer.write_bit(true)?;
        buffer.write_bit(false)?;
        buffer.write_bit(true)?;
        buffer.write_bit(false)?;

        assert_eq!(buffer.content(), &[0b1001_1110, 0b1011_1110, 0b1010_0000]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_bit_string_till_end() -> Result<(), UperError> {
        let content = &[0xFF, 0x74, 0xA6, 0x0F];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string_till_end(content, 0)?;
        assert_eq!(buffer.content(), content);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_bit_string_till_end_with_offset() -> Result<(), UperError> {
        let content = &[0b1111_1111, 0b0111_0100, 0b1010_0110, 0b0000_1111];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string_till_end(content, 7)?;
        assert_eq!(
            buffer.content(),
            &[0b1011_1010, 0b0101_0011, 0b0000_0111, 0b1000_0000]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_bit_string() -> Result<(), UperError> {
        let content = &[0b1111_1111, 0b0111_0100, 0b1010_0110, 0b0000_1111];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string(content, 7, 12)?;
        assert_eq!(buffer.content(), &[0b1011_1010, 0b0101_0000]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_length_determinant_0() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(0)?;
        assert_eq!(buffer.content(), &[0x00 | 0]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_length_determinant_1() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(1)?;
        assert_eq!(buffer.content(), &[0x00 | 1]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_length_determinant_127() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(126)?;
        assert_eq!(buffer.content(), &[0x00 | 126]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_length_determinant_128() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(128)?;
        // detects that the value is greater than 127, so
        //   10xx_xxxx xxxx_xxxx (header)
        // | --00_0000 1000_0000 (128)
        // =======================
        //   1000_0000 1000_0000
        assert_eq!(buffer.content(), &[0x80 | 0x00, 0x00 | 128]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_length_determinant_16383() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(16383)?;
        // detects that the value is greater than 127, so
        //   10xx_xxxx xxxx_xxxx (header)
        // | --11_1111 1111_1111 (16383)
        // =======================
        //   1011_1111 1111_1111
        assert_eq!(
            buffer.content(),
            &[0x80 | (16383 >> 8) as u8, 0x00 | (16383 & 0xFF) as u8]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_127() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(127)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, 127]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_128() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(128)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, 128]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_255() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(255)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, 255]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_256() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(256)?;
        // Can be represented in 2 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 2 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 2,
                ((256 & 0xFF_00) >> 8) as u8,
                (256 & 0x00_ff) as u8
            ]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_65535() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(65535)?;
        // Can be represented in 2 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 2 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 2,
                ((65535 & 0xFF_00) >> 8) as u8,
                ((65535 & 0x00_FF) >> 0) as u8
            ]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_65536() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(65536)?;
        // Can be represented in 3 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 3 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 3,
                ((65536 & 0xFF_00_00) >> 16) as u8,
                ((65536 & 0x00_FF_00) >> 8) as u8,
                ((65536 & 0x00_00_FF) >> 0) as u8,
            ]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_16777215() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(16777215)?;
        // Can be represented in 3 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 3 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 3,
                ((16777215 & 0xFF_00_00) >> 16) as u8,
                ((16777215 & 0x00_FF_00) >> 8) as u8,
                ((16777215 & 0x00_00_FF) >> 0) as u8,
            ]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_16777216() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(16777216)?;
        // Can be represented in 4 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 4 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 4,
                ((16777216 & 0xFF_00_00_00) >> 24) as u8,
                ((16777216 & 0x00_FF_00_00) >> 16) as u8,
                ((16777216 & 0x00_00_FF_00) >> 8) as u8,
                ((16777216 & 0x00_00_00_FF) >> 0) as u8,
            ]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_4294967295() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(4294967295)?;
        // Can be represented in 4 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 4 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 4,
                ((4294967295 & 0xFF_00_00_00) >> 24) as u8,
                ((4294967295 & 0x00_FF_00_00) >> 16) as u8,
                ((4294967295 & 0x00_00_FF_00) >> 8) as u8,
                ((4294967295 & 0x00_00_00_FF) >> 0) as u8,
            ]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_4294967296() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(4294967296_u64)?;
        // Can be represented in 5 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 5 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 5,
                ((4294967296_u64 & 0xFF_00_00_00_00) >> 32) as u8,
                ((4294967296_u64 & 0x00_FF_00_00_00) >> 24) as u8,
                ((4294967296_u64 & 0x00_00_FF_00_00) >> 16) as u8,
                ((4294967296_u64 & 0x00_00_00_FF_00) >> 8) as u8,
                ((4294967296_u64 & 0x00_00_00_00_FF) >> 0) as u8,
            ]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_max_i64_max() -> Result<(), UperError> {
        assert_eq!(0x7F_FF_FF_FF_FF_FF_FF_FF_u64, ::std::i64::MAX as u64);
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(0x7F_FF_FF_FF_FF_FF_FF_FF_u64)?;
        // Can be represented in 8 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 8 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 8,
                ((0x7F_FF_FF_FF_FF_FF_FF_FF_u64 & 0xFF_00_00_00_00_00_00_00) >> 56) as u8,
                ((0x7F_FF_FF_FF_FF_FF_FF_FF_u64 & 0x00_FF_00_00_00_00_00_00) >> 48) as u8,
                ((0x7F_FF_FF_FF_FF_FF_FF_FF_u64 & 0x00_00_FF_00_00_00_00_00) >> 40) as u8,
                ((0x7F_FF_FF_FF_FF_FF_FF_FF_u64 & 0x00_00_00_FF_00_00_00_00) >> 32) as u8,
                ((0x7F_FF_FF_FF_FF_FF_FF_FF_u64 & 0x00_00_00_00_FF_00_00_00) >> 24) as u8,
                ((0x7F_FF_FF_FF_FF_FF_FF_FF_u64 & 0x00_00_00_00_00_FF_00_00) >> 16) as u8,
                ((0x7F_FF_FF_FF_FF_FF_FF_FF_u64 & 0x00_00_00_00_00_00_FF_00) >> 8) as u8,
                ((0x7F_FF_FF_FF_FF_FF_FF_FF_u64 & 0x00_00_00_00_00_00_00_FF) >> 0) as u8,
            ]
        );
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_positive_only() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(buffer.write_int(0, (10, 127)), Err(UperError::ValueNotInRange(0, 10, 127)));
        // upper check
        assert_eq!(buffer.write_int(128, (10, 127)), Err(UperError::ValueNotInRange(128, 10, 127)));
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_negative() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(buffer.write_int(-11, (-10, -1)), Err(UperError::ValueNotInRange(-11, -10, -1)));
        // upper check
        assert_eq!(buffer.write_int(0, (-10, -1)), Err(UperError::ValueNotInRange(0, -10, -1)));
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_with_negative() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(buffer.write_int(-11, (-10, 1)), Err(UperError::ValueNotInRange(-11, -10,  1)));
        // upper check
        assert_eq!(buffer.write_int(2, (-10, 1)), Err(UperError::ValueNotInRange(2, -10, 1)));
    }


    #[test]
    fn bit_buffer_write_int_7bits() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int(10, (0, 127))?;
        // [0; 127] are 128 numbers, so they
        // have to fit in 7 bit
        assert_eq!(buffer.content(), &[10 << 1]);
        // be sure write_bit writes at the 8th bit
        buffer.write_bit(true);
        assert_eq!(buffer.content(), &[10 << 1 | 0b0000_0001]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_neg() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int(-10, (-128, 127))?;
        // [-128; 127] are 255 numbers, so they
        // have to fit in one byte
        assert_eq!(buffer.content(), &[118]);
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_neg_extended_range() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();
        buffer.write_int(-10, (-128, 128))?;
        // [-128; 127] are 256 numbers, so they
        // don't fit in one byte but in 9 bits
        assert_eq!(buffer.content(), &[118 >> 1, 118 << 7 | 0b0000_0000]);
        // be sure write_bit writes at the 10th bit
        buffer.write_bit(true);
        assert_eq!(buffer.content(), &[118 >> 1, 118 << 7 | 0b0100_0000]);
        Ok(())
    }
}
