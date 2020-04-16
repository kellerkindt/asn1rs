use crate::io::uper::Writer as UperWriter;
use crate::io::uper::BYTE_LEN;
use crate::io::uper::{Error as UperError, Error};
use crate::io::uper::{Reader as UperReader, Writer};
use std::iter;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Default)]
pub struct BitBuffer {
    buffer: Vec<u8>,
    write_position: usize,
    read_position: usize,
}

impl BitBuffer {
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

fn bit_string_copy_bulked(
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

impl UperReader for BitBuffer {
    fn read_substring_with_length_determinant_prefix(&mut self) -> Result<BitBuffer, Error> {
        // let the new buffer have the same bit_alignment as this current instance
        // so that ```bit_string_copy_bulked``` can utilize the fast copy-path
        let byte_len = self.read_length_determinant()?;
        let bit_len = byte_len * BYTE_LEN;
        let bit_offset = self.read_position % BYTE_LEN;

        let mut bytes = vec![0x00_u8; byte_len + if bit_offset > 0 { 1 } else { 0 }];
        self.read_bit_string(&mut bytes[..], bit_offset, bit_len)?;

        Ok(BitBuffer {
            buffer: bytes,
            write_position: bit_len + bit_offset,
            read_position: bit_offset,
        })
    }

    fn read_bit_string(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), UperError> {
        (&self.buffer[..], &mut self.read_position).read_bit_string(buffer, bit_offset, bit_length)
    }

    fn read_bit(&mut self) -> Result<bool, UperError> {
        if self.read_position < self.write_position {
            (&self.buffer[..], &mut self.read_position).read_bit()
        } else {
            Err(UperError::EndOfStream)
        }
    }
}

impl UperWriter for BitBuffer {
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

    fn write_bit(&mut self, bit: bool) -> Result<(), UperError> {
        while self.write_position + 1 > self.buffer.len() * BYTE_LEN {
            self.buffer.push(0x00);
        }
        (&mut self.buffer[..], &mut self.write_position).write_bit(bit)
    }
}
impl<'a> UperReader for (&'a [u8], &mut usize) {
    fn read_bit_string(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error> {
        bit_string_copy_bulked(&self.0[..], *self.1, buffer, bit_offset, bit_length)?;
        *self.1 += bit_length;
        Ok(())
    }

    fn read_bit(&mut self) -> Result<bool, Error> {
        if *self.1 > self.0.len() * BYTE_LEN {
            return Err(Error::EndOfStream);
        }
        let bit = self.0[*self.1 / BYTE_LEN] & (0x80 >> (*self.1 % BYTE_LEN)) != 0;
        *self.1 += 1;
        Ok(bit)
    }
}

impl<'a> UperWriter for (&'a mut [u8], &mut usize) {
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

    fn write_bit(&mut self, bit: bool) -> Result<(), UperError> {
        if *self.1 + 1 > self.0.len() * BYTE_LEN {
            return Err(Error::EndOfStream);
        }
        if bit {
            self.0[*self.1 / BYTE_LEN] |= 0x80 >> (*self.1 % BYTE_LEN);
        }
        *self.1 += 1;
        Ok(())
    }
}

#[cfg(any(test, feature = "legacy_bit_buffer"))]
#[allow(clippy::module_name_repetitions)]
pub mod legacy {
    use super::*;
    use crate::io::uper::Error as UperError;
    use crate::io::uper::Reader as UperReader;
    use crate::io::uper::Writer as UperWriter;

    pub const SIZE_BITS: usize = 100 * BYTE_LEN;

    pub struct LegacyBitBuffer<'a>(&'a mut BitBuffer);

    // the legacy BitBuffer relies solely on read_bit(), no performance optimisation
    impl UperReader for LegacyBitBuffer<'_> {
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

    fn check_int_max(buffer: &mut BitBuffer, int: u64) -> Result<(), UperError> {
        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(int, buffer2.read_int_max()?);
        }

        assert_eq!(int, buffer.read_int_max()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_0() -> Result<(), UperError> {
        const INT: u64 = 0;
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
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
        const INT: u64 = 127; // u4::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
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
        const INT: u64 = 128; // u4::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_255() -> Result<(), UperError> {
        const INT: u64 = 255; // u8::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_256() -> Result<(), UperError> {
        const INT: u64 = 256; // u8::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
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
        const INT: u64 = 65_535; // u16::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 2 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 2 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 2,
                ((INT & 0xFF_00) >> 8) as u8,
                ((INT & 0x00_FF) >> 0) as u8
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_65536() -> Result<(), UperError> {
        const INT: u64 = 65_536; // u16::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 3 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 3 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 3,
                ((INT & 0xFF_00_00) >> 16) as u8,
                ((INT & 0x00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_16777215() -> Result<(), UperError> {
        const INT: u64 = 16_777_215; // u24::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 3 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 3 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 3,
                ((INT & 0xFF_00_00) >> 16) as u8,
                ((INT & 0x00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_16777216() -> Result<(), UperError> {
        const INT: u64 = 16_777_216; // u24::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
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
        const INT: u64 = 4_294_967_295; // u32::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
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
    fn bit_buffer_int_max_4294967296() -> Result<(), UperError> {
        const INT: u64 = 4_294_967_296; // u32::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 5 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 5 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 5,
                ((INT & 0xFF_00_00_00_00) >> 32) as u8,
                ((INT & 0x00_FF_00_00_00) >> 24) as u8,
                ((INT & 0x00_00_FF_00_00) >> 16) as u8,
                ((INT & 0x00_00_00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_i64_max() -> Result<(), UperError> {
        const INT: u64 = 0x7F_FF_FF_FF_FF_FF_FF_FF_u64;
        assert_eq!(INT, i64::max_value() as u64);
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 8 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 8 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 8,
                ((INT & 0xFF_00_00_00_00_00_00_00) >> 56) as u8,
                ((INT & 0x00_FF_00_00_00_00_00_00) >> 48) as u8,
                ((INT & 0x00_00_FF_00_00_00_00_00) >> 40) as u8,
                ((INT & 0x00_00_00_FF_00_00_00_00) >> 32) as u8,
                ((INT & 0x00_00_00_00_FF_00_00_00) >> 24) as u8,
                ((INT & 0x00_00_00_00_00_FF_00_00) >> 16) as u8,
                ((INT & 0x00_00_00_00_00_00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_00_00_00_00_00_FF) >> 0) as u8,
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
        assert_eq!(
            &[0b1000_0000_, 0b1111_1111, 0b0_000_0000],
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
}
