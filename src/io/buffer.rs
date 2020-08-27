use crate::io::uper::Writer as UperWriter;
use crate::io::uper::Writer;
use crate::io::uper::BYTE_LEN;
use crate::io::uper::{Error as UperError, Error};
use std::iter;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Default)]
pub struct BitBuffer {
    pub(crate) buffer: Vec<u8>,
    pub(crate) write_position: usize,
    pub(crate) read_position: usize,
}

impl BitBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn from_bytes(buffer: Vec<u8>) -> BitBuffer {
        let bits = buffer.len() * BYTE_LEN;
        Self::from_bits(buffer, bits)
    }

    pub fn from_bits(buffer: Vec<u8>, bit_length: usize) -> BitBuffer {
        assert!(bit_length <= buffer.len() * BYTE_LEN);
        BitBuffer {
            buffer,
            write_position: bit_length,
            read_position: 0,
        }
    }

    pub fn from_bits_with_position(
        buffer: Vec<u8>,
        write_position: usize,
        read_position: usize,
    ) -> BitBuffer {
        assert!(write_position <= buffer.len() * BYTE_LEN);
        assert!(read_position <= buffer.len() * BYTE_LEN);
        BitBuffer {
            buffer,
            write_position,
            read_position,
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.write_position = 0;
        self.read_position = 0;
    }

    pub fn reset_read_position(&mut self) {
        self.read_position = 0;
    }

    pub fn content(&self) -> &[u8] {
        &self.buffer[..]
    }

    pub const fn bit_len(&self) -> usize {
        self.write_position
    }

    pub fn byte_len(&self) -> usize {
        self.buffer.len()
    }

    /// Changes the write-position to the given position for the closure call.
    /// Restores the original write-position after the call.
    ///
    /// # Panics
    /// Positions beyond the current buffer length will result in panics.
    #[inline]
    pub fn with_write_position_at<T, F: Fn(&mut Self) -> T>(&mut self, position: usize, f: F) -> T {
        debug_assert!(position <= self.buffer.len() * 8);
        let before = core::mem::replace(&mut self.write_position, position);
        let result = f(self);
        self.write_position = before;
        result
    }

    /// Changes the read-position to the given position for the closure call.
    /// Restores the original read-position after the call.
    ///
    /// # Panics
    /// Positions beyond the current write-position will result in panics.
    #[inline]
    pub fn with_read_position_at<T, F: Fn(&mut Self) -> T>(&mut self, position: usize, f: F) -> T {
        debug_assert!(position < self.write_position);
        let before = core::mem::replace(&mut self.read_position, position);
        let result = f(self);
        self.read_position = before;
        result
    }
}

fn bit_string_copy(
    src: &[u8],
    src_bit_position: usize,
    dst: &mut [u8],
    dst_bit_position: usize,
    len: usize,
) -> Result<(), UperError> {
    if dst.len() * BYTE_LEN < dst_bit_position + len {
        return Err(Error::InsufficientSpaceInDestinationBuffer);
    }
    if src.len() * BYTE_LEN < src_bit_position + len {
        return Err(Error::InsufficientDataInSourceBuffer);
    }
    for bit in 0..len {
        let dst_byte_pos = (dst_bit_position + bit) / BYTE_LEN;
        let dst_bit_pos = (dst_bit_position + bit) % BYTE_LEN;
        let dst_bit_pos = BYTE_LEN - dst_bit_pos - 1; // flip

        let bit = {
            let src_byte_pos = (src_bit_position + bit) / BYTE_LEN;
            let src_bit_pos = (src_bit_position + bit) % BYTE_LEN;
            let src_bit_pos = BYTE_LEN - src_bit_pos - 1; // flip

            src[src_byte_pos] & (0x01 << src_bit_pos) > 0
        };

        if bit {
            // set bit
            dst[dst_byte_pos] |= 0x01 << dst_bit_pos;
        } else {
            // reset bit
            dst[dst_byte_pos] &= !(0x01 << dst_bit_pos);
        }
    }
    Ok(())
}

pub(crate) fn bit_string_copy_bulked(
    src: &[u8],
    src_bit_position: usize,
    dst: &mut [u8],
    dst_bit_position: usize,
    len: usize,
) -> Result<(), UperError> {
    // chosen by real world tests
    if len <= BYTE_LEN * 2 {
        return bit_string_copy(src, src_bit_position, dst, dst_bit_position, len);
    }

    if dst.len() * BYTE_LEN < dst_bit_position + len {
        return Err(Error::InsufficientSpaceInDestinationBuffer);
    }
    if src.len() * BYTE_LEN < src_bit_position + len {
        return Err(Error::InsufficientDataInSourceBuffer);
    }

    let bits_till_full_byte_src = (BYTE_LEN - (src_bit_position % BYTE_LEN)) % BYTE_LEN;

    // align read_position to a full byte
    if bits_till_full_byte_src != 0 {
        bit_string_copy(
            src,
            src_bit_position,
            dst,
            dst_bit_position,
            bits_till_full_byte_src.min(len),
        )?;

        if len <= bits_till_full_byte_src {
            return Ok(());
        }
    }

    let src_bit_position = src_bit_position + bits_till_full_byte_src;
    let dst_bit_position = dst_bit_position + bits_till_full_byte_src;

    let len = len - bits_till_full_byte_src;

    let dst_byte_index = dst_bit_position / BYTE_LEN;
    let dst_byte_offset = dst_bit_position % BYTE_LEN;

    let src_byte_index = src_bit_position / BYTE_LEN;
    let len_in_bytes = len / BYTE_LEN;

    if dst_byte_offset == 0 {
        // both align
        dst[dst_byte_index..dst_byte_index + len_in_bytes]
            .copy_from_slice(&src[src_byte_index..src_byte_index + len_in_bytes]);
    } else {
        for index in 0..len_in_bytes {
            let byte = src[index + src_byte_index];
            let half_left = byte >> dst_byte_offset;
            let half_right = byte << (BYTE_LEN - dst_byte_offset);

            dst[index + dst_byte_index] = (dst[index + dst_byte_index]
                    & (0xFF << (BYTE_LEN - dst_byte_offset))) // do not destroy current values on the furthe left side
                    | half_left;

            dst[index + dst_byte_index + 1] = half_right;
        }
    }

    if len % BYTE_LEN == 0 {
        Ok(())
    } else {
        // copy the remaining
        bit_string_copy(
            src,
            src_bit_position + (len_in_bytes * BYTE_LEN),
            dst,
            dst_bit_position + (len_in_bytes * BYTE_LEN),
            len % BYTE_LEN,
        )
    }
}

impl Into<Vec<u8>> for BitBuffer {
    fn into(self) -> Vec<u8> {
        self.buffer
    }
}

impl From<Vec<u8>> for BitBuffer {
    fn from(buffer: Vec<u8>) -> Self {
        Self::from_bytes(buffer)
    }
}

impl UperWriter for BitBuffer {
    #[inline]
    fn write_substring_with_length_determinant_prefix(
        &mut self,
        fun: &dyn Fn(&mut dyn Writer) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut buffer = BitBuffer::default();
        let bit_offset = self.write_position % BYTE_LEN;
        // let the new buffer have the same bit_alignment as this current instance
        // so that ```bit_string_copy_bulked``` can utilize the fast copy-path
        buffer.write_bit_string(&[0x00_u8], 0, bit_offset)?;
        fun(&mut buffer)?;
        let byte_len = (buffer.bit_len() - bit_offset + (BYTE_LEN - 1)) / BYTE_LEN;
        self.write_length_determinant(byte_len)?;
        self.write_bit_string(buffer.content(), bit_offset, buffer.bit_len() - bit_offset)?;
        Ok(())
    }

    #[inline]
    fn write_bit_string(
        &mut self,
        buffer: &[u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), UperError> {
        let bytes_together = (self.write_position + bit_length + (BYTE_LEN - 1)) / BYTE_LEN;
        if bytes_together > self.buffer.len() {
            self.buffer
                .extend(iter::repeat(0x00).take(bytes_together - self.buffer.len()));
        }

        (&mut self.buffer[..], &mut self.write_position)
            .write_bit_string(buffer, bit_offset, bit_length)
    }

    #[inline]
    fn write_bit(&mut self, bit: bool) -> Result<(), UperError> {
        while self.write_position + 1 > self.buffer.len() * BYTE_LEN {
            self.buffer.push(0x00);
        }
        (&mut self.buffer[..], &mut self.write_position).write_bit(bit)
    }
}

impl<'a> UperWriter for (&'a mut [u8], &mut usize) {
    #[inline]
    fn write_bit_string(
        &mut self,
        buffer: &[u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), UperError> {
        bit_string_copy_bulked(buffer, bit_offset, &mut self.0[..], *self.1, bit_length)?;
        *self.1 += bit_length;
        Ok(())
    }

    #[inline]
    fn write_bit(&mut self, bit: bool) -> Result<(), UperError> {
        if *self.1 + 1 > self.0.len() * BYTE_LEN {
            return Err(Error::EndOfStream);
        }
        if bit {
            self.0[*self.1 / BYTE_LEN] |= 0x80 >> (*self.1 % BYTE_LEN);
        } else {
            self.0[*self.1 / BYTE_LEN] &= !(0x80 >> (*self.1 % BYTE_LEN));
        }
        *self.1 += 1;
        Ok(())
    }
}

#[cfg(any(test, feature = "legacy_bit_buffer"))]
#[allow(clippy::module_name_repetitions, deprecated)]
pub mod legacy {
    use super::*;
    use crate::io::uper::Reader as UperReader;
    use crate::io::uper::Writer as UperWriter;
    use crate::io::uper::{Error as UperError, UPER_LENGTH_DET_L1, UPER_LENGTH_DET_L2};
    use byteorder::ByteOrder;
    use byteorder::NetworkEndian;

    pub const SIZE_BITS: usize = 100 * BYTE_LEN;

    pub struct LegacyBitBuffer<'a>(&'a mut BitBuffer);

    // the legacy BitBuffer relies solely on read_bit(), no performance optimisation
    impl UperReader for LegacyBitBuffer<'_> {
        fn read_utf8_string(&mut self) -> Result<String, Error> {
            let len = self.read_length_determinant()?;
            let mut buffer = vec![0_u8; len];
            self.read_bit_string_till_end(&mut buffer[..len], 0)?;
            if let Ok(string) = String::from_utf8(buffer) {
                Ok(string)
            } else {
                Err(Error::InvalidUtf8String)
            }
        }

        fn read_int(&mut self, range: (i64, i64)) -> Result<i64, Error> {
            let (lower, upper) = range;
            let leading_zeros = ((upper - lower) as u64).leading_zeros();

            let mut buffer = [0_u8; 8];
            let buffer_bits = buffer.len() * BYTE_LEN as usize;
            debug_assert!(buffer_bits == 64);
            self.read_bit_string_till_end(&mut buffer[..], leading_zeros as usize)?;
            let value = NetworkEndian::read_u64(&buffer[..]) as i64;
            Ok(value + lower)
        }

        fn read_int_normally_small(&mut self) -> Result<u64, Error> {
            // X.691-201508 11.6
            let is_small = !self.read_bit()?;
            if is_small {
                // 11.6.1: 6 bit of the number
                let mut buffer = [0u8; std::mem::size_of::<u64>()];
                self.read_bit_string(&mut buffer[7..8], 2, 6)?;
                Ok(u64::from_be_bytes(buffer))
            } else {
                // 11.6.2: (length-determinant + number)
                // this cannot be negative... logically
                let value = self.read_int_max_unsigned()?;
                // u64::try_from(value).map_err(|_| Error::ValueIsNegativeButExpectedUnsigned(value))
                Ok(value)
            }
        }

        fn read_int_max_signed(&mut self) -> Result<i64, Error> {
            let len_in_bytes = self.read_length_determinant()?;
            if len_in_bytes > std::mem::size_of::<i64>() {
                Err(Error::UnsupportedOperation(
                    "Reading bigger data types than 64bit is not supported".into(),
                ))
            } else {
                let mut buffer = [0_u8; std::mem::size_of::<i64>()];
                let offset = (buffer.len() - len_in_bytes) * BYTE_LEN;
                self.read_bit_string_till_end(&mut buffer[..], offset)?;
                let sign_position = buffer.len() - len_in_bytes;
                if buffer[sign_position] & 0x80 != 0 {
                    for value in buffer.iter_mut().take(sign_position) {
                        *value = 0xFF;
                    }
                }
                Ok(i64::from_be_bytes(buffer))
            }
        }

        fn read_int_max_unsigned(&mut self) -> Result<u64, Error> {
            let len_in_bytes = self.read_length_determinant()?;
            if len_in_bytes > std::mem::size_of::<u64>() {
                Err(Error::UnsupportedOperation(
                    "Reading bigger data types than 64bit is not supported".into(),
                ))
            } else {
                let mut buffer = [0_u8; std::mem::size_of::<u64>()];
                let offset = (buffer.len() - len_in_bytes) * BYTE_LEN;
                self.read_bit_string_till_end(&mut buffer[..], offset)?;
                Ok(u64::from_be_bytes(buffer))
            }
        }

        fn read_bit_string(
            &mut self,
            buffer: &mut [u8],
            bit_offset: usize,
            bit_length: usize,
        ) -> Result<(), Error> {
            if buffer.len() * BYTE_LEN < bit_offset
                || buffer.len() * BYTE_LEN < bit_offset + bit_length
            {
                return Err(Error::InsufficientSpaceInDestinationBuffer);
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

        fn read_octet_string(
            &mut self,
            length_range: Option<(i64, i64)>,
        ) -> Result<Vec<u8>, Error> {
            let len = if let Some((min, max)) = length_range {
                self.read_int((min, max))? as usize
            } else {
                self.read_length_determinant()?
            };
            let mut vec = vec![0_u8; len];
            self.read_bit_string_till_end(&mut vec[..], 0)?;
            Ok(vec)
        }

        fn read_bit_string_till_end(
            &mut self,
            buffer: &mut [u8],
            bit_offset: usize,
        ) -> Result<(), Error> {
            let len = (buffer.len() * BYTE_LEN) - bit_offset;
            self.read_bit_string(buffer, bit_offset, len)
        }

        fn read_length_determinant(&mut self) -> Result<usize, Error> {
            if !self.read_bit()? {
                // length <= UPER_LENGTH_DET_L1
                Ok(self.read_int((0, UPER_LENGTH_DET_L1))? as usize)
            } else if !self.read_bit()? {
                // length <= UPER_LENGTH_DET_L2
                Ok(self.read_int((0, UPER_LENGTH_DET_L2))? as usize)
            } else {
                Err(Error::UnsupportedOperation(
                    "Cannot read length determinant for other than i8 and i16".into(),
                ))
            }
        }

        fn read_bit(&mut self) -> Result<bool, UperError> {
            self.0.read_bit()
        }
    }

    // the legacy BitBuffer relies solely on write_bit(), no performance optimisation
    impl UperWriter for LegacyBitBuffer<'_> {
        fn write_bit(&mut self, bit: bool) -> Result<(), UperError> {
            self.0.write_bit(bit)
        }
    }

    pub fn bit_buffer(size: usize, pos: usize) -> (BitBuffer, Vec<u8>, BitBuffer) {
        let mut bits = BitBuffer::from(vec![
            0b0101_0101_u8.wrapping_shl(pos as u32 % 2);
            size + if pos > 0 { 1 } else { 0 }
        ]);
        for _ in 0..pos {
            bits.read_bit().unwrap();
        }
        (
            bits,
            vec![0_u8; size + if pos > 0 { 1 } else { 0 }],
            BitBuffer::from_bits_with_position(
                vec![0_u8; size + if pos > 0 { 1 } else { 0 }],
                pos,
                pos,
            ),
        )
    }

    pub fn check_result(bits: &mut BitBuffer, offset: usize, len: usize) {
        for i in 0..offset {
            assert!(
                !bits.read_bit().unwrap(),
                "Failed on offset with i={}, offset={}, bits={:?}",
                i,
                offset,
                bits
            );
        }
        for i in 0..len {
            assert_eq!(
                i % 2 == 1,
                bits.read_bit().unwrap(),
                "Failed on data with i={}, offset={}, bits={:?}",
                i,
                offset,
                bits
            );
        }
    }

    pub fn legacy_bit_buffer(size_bits: usize, offset: usize, pos: usize) -> (Vec<u8>, BitBuffer) {
        let (mut bits, mut dest, mut write) = bit_buffer(
            (size_bits + (BYTE_LEN - 1)) / BYTE_LEN + if offset > 0 { 1 } else { 0 },
            pos,
        );
        LegacyBitBuffer(&mut bits)
            .read_bit_string(&mut dest[..], offset, size_bits)
            .unwrap();
        LegacyBitBuffer(&mut write)
            .write_bit_string(&dest[..], offset, size_bits)
            .unwrap();
        (dest, write)
    }

    pub fn new_bit_buffer(size_bits: usize, offset: usize, pos: usize) -> (Vec<u8>, BitBuffer) {
        let (mut bits, mut dest, mut write) = bit_buffer(
            (size_bits + (BYTE_LEN - 1)) / BYTE_LEN + if offset > 0 { 1 } else { 0 },
            pos,
        );
        bits.read_bit_string(&mut dest[..], offset, size_bits)
            .unwrap();
        write
            .write_bit_string(&dest[..], offset, size_bits)
            .unwrap();
        (dest, write)
    }

    pub fn legacy_bit_buffer_with_check(size_bits: usize, offset: usize, pos: usize) {
        let (bits, mut written) = legacy_bit_buffer(size_bits, offset, pos);
        check_result(&mut BitBuffer::from(bits), offset, size_bits);
        check_result(&mut written, 0, size_bits);
    }

    pub fn new_bit_buffer_with_check(size_bits: usize, offset: usize, pos: usize) {
        let (bits, mut written) = new_bit_buffer(size_bits, offset, pos);
        check_result(&mut BitBuffer::from(bits), offset, size_bits);
        check_result(&mut written, 0, size_bits);
    }
}

#[allow(clippy::identity_op)] // for better readability across multiple tests
#[cfg(test)]
mod tests {
    use super::legacy::*;
    use super::*;
    use crate::io::uper::Reader as UperReader;

    #[test]
    fn test_legacy_bit_string_offset_0_to_7_pos_0_to_7() {
        for offset in 0..BYTE_LEN {
            for pos in 0..BYTE_LEN {
                legacy_bit_buffer_with_check(SIZE_BITS, offset, pos)
            }
        }
    }

    #[test]
    fn test_new_bit_string_offset_0_to_7_pos_0_to_7() {
        for offset in 0..BYTE_LEN {
            for pos in 0..BYTE_LEN {
                new_bit_buffer_with_check(SIZE_BITS, offset, pos)
            }
        }
    }

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

        let mut buffer = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);
        assert!(!buffer.read_bit()?);
        assert!(buffer.read_bit()?);

        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);

        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);

        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);

        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);

        assert_eq!(buffer.read_bit(), Err(UperError::EndOfStream));

        Ok(())
    }

    #[test]
    fn bit_buffer_bit_string_till_end() -> Result<(), UperError> {
        let content = &[0xFF, 0x74, 0xA6, 0x0F];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string_till_end(content, 0)?;
        assert_eq!(buffer.content(), content);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            let mut content2 = vec![0_u8; content.len()];
            buffer2.read_bit_string_till_end(&mut content2[..], 0)?;
            assert_eq!(&content[..], &content2[..]);
        }

        let mut content2 = vec![0xFF_u8; content.len()];
        buffer.read_bit_string_till_end(&mut content2[..], 0)?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffer_bit_string_till_end_with_offset() -> Result<(), UperError> {
        let content = &[0b1111_1111, 0b0111_0100, 0b1010_0110, 0b0000_1111];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string_till_end(content, 7)?;
        assert_eq!(
            buffer.content(),
            &[0b1011_1010, 0b0101_0011, 0b0000_0111, 0b1000_0000]
        );

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            let mut content2 = vec![0xFF_u8; content.len()];
            content2[0] = content[0] & 0b1111_1110; // since we are skipping the first 7 bits
            buffer2.read_bit_string_till_end(&mut content2[..], 7)?;
            assert_eq!(&content[..], &content2[..]);
        }

        let mut content2 = vec![0_u8; content.len()];
        content2[0] = content[0] & 0b1111_1110; // since we are skipping the first 7 bits
        buffer.read_bit_string_till_end(&mut content2[..], 7)?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffer_bit_string() -> Result<(), UperError> {
        let content = &[0b1111_1111, 0b0111_0100, 0b1010_0110, 0b0000_1111];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string(content, 7, 12)?;
        assert_eq!(buffer.content(), &[0b1011_1010, 0b0101_0000]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            let mut content2 = vec![0_u8; content.len()];
            // since we are skipping the first 7 bits
            let content = &[
                content[0] & 0x01,
                content[1],
                content[2] & 0b1110_0000,
                0x00,
            ];
            buffer2.read_bit_string(&mut content2[..], 7, 12)?;
            assert_eq!(&content[..], &content2[..]);
        }

        let mut content2 = vec![0x00_u8; content.len()];
        // since we are skipping the first 7 bits
        let content = &[
            content[0] & 0x01,
            content[1],
            content[2] & 0b1110_0000,
            0x00,
        ];
        buffer.read_bit_string(&mut content2[..], 7, 12)?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_0() -> Result<(), UperError> {
        const DET: usize = 0;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);

        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_1() -> Result<(), UperError> {
        const DET: usize = 1;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_127() -> Result<(), UperError> {
        const DET: usize = 126;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_128() -> Result<(), UperError> {
        const DET: usize = 128;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        // detects that the value is greater than 127, so
        //   10xx_xxxx xxxx_xxxx (header)
        // | --00_0000 1000_0000 (128)
        // =======================
        //   1000_0000 1000_0000
        assert_eq!(buffer.content(), &[0x80 | 0x00, 0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_16383() -> Result<(), UperError> {
        const DET: usize = 16383;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        // detects that the value is greater than 127, so
        //   10xx_xxxx xxxx_xxxx (header)
        // | --11_1111 1111_1111 (16383)
        // =======================
        //   1011_1111 1111_1111
        assert_eq!(
            buffer.content(),
            &[0x80 | (DET >> 8) as u8, 0x00 | (DET & 0xFF) as u8]
        );

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);
        Ok(())
    }

    fn check_int_max(buffer: &mut BitBuffer, int: i64) -> Result<(), UperError> {
        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(int, buffer2.read_int_max_signed()?);
        }

        assert_eq!(int, buffer.read_int_max_signed()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_neg_12() -> Result<(), UperError> {
        const INT: i64 = -12;
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_0() -> Result<(), UperError> {
        const INT: i64 = 0;
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_127() -> Result<(), UperError> {
        const INT: i64 = 127; // u4::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_128() -> Result<(), UperError> {
        const INT: i64 = 128; // u4::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        assert_eq!(buffer.content(), &[0x02, 0x00, 0x80]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_255() -> Result<(), UperError> {
        const INT: i64 = 255; // u8::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        assert_eq!(buffer.content(), &[0x02, 0x00, 0xFF]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_256() -> Result<(), UperError> {
        const INT: i64 = 256; // u8::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        // Can be represented in 2 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 2 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 2,
                ((INT & 0xFF_00) >> 8) as u8,
                ((INT & 0x00_ff) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_65535() -> Result<(), UperError> {
        const INT: i64 = 65_535; // u16::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        assert_eq!(buffer.content(), &[0x03, 0x00, 0xFF, 0xFF]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_65536() -> Result<(), UperError> {
        const INT: i64 = 65_536; // u16::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        assert_eq!(buffer.content(), &[0x03, 0x01, 0x00, 0x00]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_16777215() -> Result<(), UperError> {
        const INT: i64 = 16_777_215; // u24::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        assert_eq!(buffer.content(), &[0x04, 0x00, 0xFF, 0xFF, 0xFF]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_16777216() -> Result<(), UperError> {
        const INT: i64 = 16_777_216; // u24::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        // Can be represented in 4 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 4 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 4,
                ((INT & 0xFF_00_00_00) >> 24) as u8,
                ((INT & 0x00_FF_00_00) >> 16) as u8,
                ((INT & 0x00_00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_4294967295() -> Result<(), UperError> {
        const INT: i64 = 4_294_967_295; // u32::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        assert_eq!(buffer.content(), &[0x05, 0x00, 0xFF, 0xFF, 0xFF, 0xFF]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_4294967296() -> Result<(), UperError> {
        const INT: i64 = 4_294_967_296; // u32::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        assert_eq!(buffer.content(), &[0x05, 0x01, 0x00, 0x00, 0x00, 0x00]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_i64_max() -> Result<(), UperError> {
        const INT: i64 = i64::max_value();
        let mut buffer = BitBuffer::default();
        buffer.write_int_max_signed(INT)?;
        // Can be represented in 8 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 8 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 8,
                ((INT as u64 & 0xFF_00_00_00_00_00_00_00_u64) >> 56) as u8,
                ((INT as u64 & 0x00_FF_00_00_00_00_00_00_u64) >> 48) as u8,
                ((INT as u64 & 0x00_00_FF_00_00_00_00_00_u64) >> 40) as u8,
                ((INT as u64 & 0x00_00_00_FF_00_00_00_00_u64) >> 32) as u8,
                ((INT as u64 & 0x00_00_00_00_FF_00_00_00_u64) >> 24) as u8,
                ((INT as u64 & 0x00_00_00_00_00_FF_00_00_u64) >> 16) as u8,
                ((INT as u64 & 0x00_00_00_00_00_00_FF_00_u64) >> 8) as u8,
                ((INT as u64 & 0x00_00_00_00_00_00_00_FF_u64) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_positive_only() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_int(0, (10, 127)),
            Err(UperError::ValueNotInRange(0, 10, 127))
        );
        // upper check
        assert_eq!(
            buffer.write_int(128, (10, 127)),
            Err(UperError::ValueNotInRange(128, 10, 127))
        );
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_negative() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_int(-11, (-10, -1)),
            Err(UperError::ValueNotInRange(-11, -10, -1))
        );
        // upper check
        assert_eq!(
            buffer.write_int(0, (-10, -1)),
            Err(UperError::ValueNotInRange(0, -10, -1))
        );
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_with_negative() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_int(-11, (-10, 1)),
            Err(UperError::ValueNotInRange(-11, -10, 1))
        );
        // upper check
        assert_eq!(
            buffer.write_int(2, (-10, 1)),
            Err(UperError::ValueNotInRange(2, -10, 1))
        );
    }

    fn check_int(buffer: &mut BitBuffer, int: i64, range: (i64, i64)) -> Result<(), UperError> {
        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(int, buffer2.read_int(range)?);
        }
        assert_eq!(int, buffer.read_int(range)?);
        Ok(())
    }

    #[test]
    fn bit_buffer_int_7bits() -> Result<(), UperError> {
        const INT: i64 = 10;
        const RANGE: (i64, i64) = (0, 127);
        let mut buffer = BitBuffer::default();
        buffer.write_int(INT, RANGE)?;
        // [0; 127] are 128 numbers, so they
        // have to fit in 7 bit
        assert_eq!(buffer.content(), &[(INT as u8) << 1]);
        check_int(&mut buffer, INT, RANGE)?;
        // be sure write_bit writes at the 8th bit
        buffer.write_bit(true)?;
        assert_eq!(buffer.content(), &[(INT as u8) << 1 | 0b0000_0001]);
        Ok(())
    }

    #[test]
    fn bit_buffer_int_neg() -> Result<(), UperError> {
        const INT: i64 = -10;
        const RANGE: (i64, i64) = (-128, 127);
        let mut buffer = BitBuffer::default();
        buffer.write_int(INT, RANGE)?;
        // [-128; 127] are 255 numbers, so they
        // have to fit in one byte
        assert_eq!(buffer.content(), &[(INT - RANGE.0) as u8]);
        check_int(&mut buffer, INT, RANGE)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_neg_extended_range() -> Result<(), UperError> {
        const INT: i64 = -10;
        const RANGE: (i64, i64) = (-128, 128);
        let mut buffer = BitBuffer::default();
        buffer.write_int(INT, RANGE)?;
        // [-128; 127] are 256 numbers, so they
        // don't fit in one byte but in 9 bits
        assert_eq!(
            buffer.content(),
            &[
                ((INT - RANGE.0) as u8) >> 1,
                (((INT - RANGE.0) as u8) << 7) | 0b0000_0000
            ]
        );
        // be sure write_bit writes at the 10th bit
        buffer.write_bit(true)?;
        assert_eq!(
            buffer.content(),
            &[
                ((INT - RANGE.0) as u8) >> 1,
                ((INT - RANGE.0) as u8) << 7 | 0b0100_0000
            ]
        );
        check_int(&mut buffer, INT, RANGE)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_with_range() -> Result<(), UperError> {
        // test scenario from https://github.com/alexvoronov/geonetworking/blob/57a43113aeabc25f005ea17f76409aed148e67b5/camdenm/src/test/java/net/gcdc/camdenm/UperEncoderDecodeTest.java#L169
        const BYTES: &[u8] = &[0x2A, 0x2B, 0x96, 0xFF];
        const RANGE: (i64, i64) = (1, 20);
        let mut buffer = BitBuffer::default();
        buffer.write_octet_string(BYTES, Some(RANGE))?;
        assert_eq!(&[0x19, 0x51, 0x5c, 0xb7, 0xf8], &buffer.content(),);
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_without_range() -> Result<(), UperError> {
        const BYTES: &[u8] = &[0x2A, 0x2B, 0x96, 0xFF];
        let mut buffer = BitBuffer::default();
        buffer.write_octet_string(BYTES, None)?;
        assert_eq!(&[0x04, 0x2a, 0x2b, 0x96, 0xff], &buffer.content(),);
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_empty() -> Result<(), UperError> {
        const BYTES: &[u8] = &[];
        let mut buffer = BitBuffer::default();
        buffer.write_octet_string(BYTES, None)?;
        assert_eq!(&[0x00], &buffer.content(),);
        Ok(())
    }

    #[test]
    fn test_int_normally_small_5() -> Result<(), UperError> {
        // example from larmouth-asn1-book, p.296, Figure III-25
        let mut buffer = BitBuffer::default();
        buffer.write_int_normally_small(5)?;
        // first 7 bits are relevant
        assert_eq!(&[0b0000_101_0], &buffer.content());
        assert_eq!(5, buffer.read_int_normally_small()?);
        Ok(())
    }

    #[test]
    fn test_int_normally_small_60() -> Result<(), UperError> {
        // example from larmouth-asn1-book, p.296, Figure III-25
        let mut buffer = BitBuffer::default();
        buffer.write_int_normally_small(60)?;
        // first 7 bits
        assert_eq!(&[0b0111_100_0], &buffer.content());
        assert_eq!(60, buffer.read_int_normally_small()?);
        Ok(())
    }

    #[test]
    fn test_int_normally_small_254() -> Result<(), UperError> {
        // example from larmouth-asn1-book, p.296, Figure III-25
        let mut buffer = BitBuffer::default();
        buffer.write_int_normally_small(254)?;
        // first 17 bits are relevant
        // assert_eq!(&[0x1, 0b0000_0001, 0b1111_1110], &buffer.content());
        assert_eq!(
            //  Bit for greater 63
            //  |
            //  V |-len 1 byte-| |-value 254-| |-rest-|
            &[0b1_000_0000, 0b1__111_1111, 0b0_000_0000],
            &buffer.content()
        );
        Ok(())
    }

    #[test]
    fn test_write_choice_index_extensible() -> Result<(), UperError> {
        fn write_once(
            index: u64,
            no_of_default_variants: u64,
        ) -> Result<(usize, Vec<u8>), UperError> {
            let mut buffer = BitBuffer::default();
            buffer.write_choice_index_extensible(index, no_of_default_variants)?;
            let bits = buffer.bit_len();
            Ok((bits, buffer.into()))
        }
        assert_eq!((2, vec![0x00]), write_once(0, 2)?);
        assert_eq!((2, vec![0x40]), write_once(1, 2)?);
        assert_eq!((8, vec![0x80]), write_once(2, 2)?);
        assert_eq!((8, vec![0x81]), write_once(3, 2)?);
        Ok(())
    }

    #[test]
    fn test_read_choice_index_extensible() -> Result<(), UperError> {
        fn read_once(data: &[u8], bits: usize, no_of_variants: u64) -> Result<u64, UperError> {
            let mut buffer = BitBuffer::default();
            buffer.write_bit_string(data, 0, bits)?;
            buffer.read_choice_index_extensible(no_of_variants)
        }
        assert_eq!(0, read_once(&[0x00], 2, 2)?);
        assert_eq!(1, read_once(&[0x40], 2, 2)?);
        assert_eq!(2, read_once(&[0x80], 8, 2)?);
        assert_eq!(3, read_once(&[0x81], 8, 2)?);
        Ok(())
    }

    #[test]
    fn test_sub_string_with_length_delimiter_prefix() {
        let mut buffer = BitBuffer::default();
        buffer
            .write_substring_with_length_determinant_prefix(&|writer| {
                writer.write_int_max_signed(1337)
            })
            .unwrap();
        assert_eq!(&[0x03, 0x02, 0x05, 0x39], buffer.content());
        let mut inner = buffer
            .read_substring_with_length_determinant_prefix()
            .unwrap();
        assert_eq!(1337, inner.read_int_max_signed().unwrap());
    }

    #[test]
    fn test_sub_string_with_length_delimiter_prefix_not_aligned() {
        let mut buffer = BitBuffer::default();
        buffer.write_bit(false).unwrap();
        buffer.write_bit(false).unwrap();
        buffer.write_bit(false).unwrap();
        buffer.write_bit(false).unwrap();
        buffer
            .write_substring_with_length_determinant_prefix(&|writer| {
                writer.write_int_max_signed(1337)
            })
            .unwrap();
        assert_eq!(&[0x00, 0x30, 0x20, 0x53, 0x90], buffer.content());
        assert_eq!(false, buffer.read_bit().unwrap());
        assert_eq!(false, buffer.read_bit().unwrap());
        assert_eq!(false, buffer.read_bit().unwrap());
        assert_eq!(false, buffer.read_bit().unwrap());
        let mut inner = buffer
            .read_substring_with_length_determinant_prefix()
            .unwrap();
        assert_eq!(1337, inner.read_int_max_signed().unwrap());
    }
    #[test]
    fn test_sub_string_with_length_delimiter_prefix_raw_not_aligned() {
        let mut buffer = ([0_u8; 1024], 0_usize);
        let writer = &mut (&mut buffer.0[..], &mut buffer.1) as &mut dyn UperWriter;
        writer.write_bit(false).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(false).unwrap();
        writer
            .write_substring_with_length_determinant_prefix(&|writer| {
                writer.write_int_max_signed(1337)
            })
            .unwrap();
        assert_eq!(&[0x00, 0x30, 0x20, 0x53, 0x90], &buffer.0[..5]);
        buffer.1 = 0;
        let reader = &mut (&buffer.0[..], &mut buffer.1) as &mut dyn UperReader;
        assert_eq!(false, reader.read_bit().unwrap());
        assert_eq!(false, reader.read_bit().unwrap());
        assert_eq!(false, reader.read_bit().unwrap());
        assert_eq!(false, reader.read_bit().unwrap());
        let mut inner = reader
            .read_substring_with_length_determinant_prefix()
            .unwrap();
        assert_eq!(1337, inner.read_int_max_signed().unwrap());
    }
}
