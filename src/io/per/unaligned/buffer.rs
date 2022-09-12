use super::*;
use crate::io::per::Error;
use crate::io::per::ErrorKind;

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

    pub fn from_bytes(buffer: Vec<u8>) -> Self {
        let bits = buffer.len() * BYTE_LEN;
        Self::from_bits(buffer, bits)
    }

    pub fn from_bits(buffer: Vec<u8>, bit_length: usize) -> Self {
        assert!(bit_length <= buffer.len() * BYTE_LEN);
        Self {
            buffer,
            write_position: bit_length,
            read_position: 0,
        }
    }

    pub fn from_bits_with_position(
        buffer: Vec<u8>,
        write_position: usize,
        read_position: usize,
    ) -> Self {
        assert!(write_position <= buffer.len() * BYTE_LEN);
        assert!(read_position <= buffer.len() * BYTE_LEN);
        Self {
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
        &self.buffer
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

    /// Sets the `write_position` to `read_position + max_read_len` for the call of the given
    /// closure
    pub fn with_max_read<T, F: Fn(&mut Self) -> T>(&mut self, max_read_len: usize, f: F) -> T {
        let before =
            core::mem::replace(&mut self.write_position, self.read_position + max_read_len);
        let result = f(self);
        self.write_position = before;
        result
    }

    pub fn ensure_can_write_additional_bits(&mut self, bit_len: usize) {
        if self.write_position + bit_len >= self.buffer.len() * BYTE_LEN {
            let required_len = ((self.write_position + bit_len) + 7) / BYTE_LEN;
            let extend_by_len = required_len - self.buffer.len();
            self.buffer
                .extend(core::iter::repeat(0u8).take(extend_by_len))
        }
    }
}

impl From<BitBuffer> for Vec<u8> {
    fn from(bb: BitBuffer) -> Vec<u8> {
        bb.buffer
    }
}

impl From<Vec<u8>> for BitBuffer {
    fn from(buffer: Vec<u8>) -> Self {
        Self::from_bytes(buffer)
    }
}

impl BitRead for BitBuffer {
    #[inline]
    fn read_bit(&mut self) -> Result<bool, Error> {
        if self.read_position < self.write_position {
            BitRead::read_bit(&mut (&self.buffer[..], &mut self.read_position))
        } else {
            Err(ErrorKind::EndOfStream.into())
        }
    }

    #[inline]
    fn read_bits(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        BitRead::read_bits(&mut (&self.buffer[..], &mut self.read_position), dst)
    }

    #[inline]
    fn read_bits_with_offset(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
    ) -> Result<(), Error> {
        BitRead::read_bits_with_offset(
            &mut (&self.buffer[..], &mut self.read_position),
            dst,
            dst_bit_offset,
        )
    }

    #[inline]
    fn read_bits_with_len(&mut self, dst: &mut [u8], dst_bit_len: usize) -> Result<(), Error> {
        BitRead::read_bits_with_len(
            &mut (&self.buffer[..], &mut self.read_position),
            dst,
            dst_bit_len,
        )
    }

    #[inline]
    fn read_bits_with_offset_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
        dst_bit_len: usize,
    ) -> Result<(), Error> {
        BitRead::read_bits_with_offset_len(
            &mut (&self.buffer[..], &mut self.read_position),
            dst,
            dst_bit_offset,
            dst_bit_len,
        )
    }
}

impl BitWrite for BitBuffer {
    #[inline]
    fn write_bit(&mut self, bit: bool) -> Result<(), Error> {
        self.ensure_can_write_additional_bits(1);
        BitWrite::write_bit(&mut (&mut self.buffer[..], &mut self.write_position), bit)
    }

    #[inline]
    fn write_bits(&mut self, src: &[u8]) -> Result<(), Error> {
        self.ensure_can_write_additional_bits(src.len() * BYTE_LEN);
        BitWrite::write_bits(&mut (&mut self.buffer[..], &mut self.write_position), src)
    }

    #[inline]
    fn write_bits_with_offset(&mut self, src: &[u8], src_bit_offset: usize) -> Result<(), Error> {
        self.ensure_can_write_additional_bits(src.len() * BYTE_LEN - src_bit_offset);
        BitWrite::write_bits_with_offset(
            &mut (&mut self.buffer[..], &mut self.write_position),
            src,
            src_bit_offset,
        )
    }

    #[inline]
    fn write_bits_with_len(&mut self, src: &[u8], bit_len: usize) -> Result<(), Error> {
        self.ensure_can_write_additional_bits(bit_len);
        BitWrite::write_bits_with_len(
            &mut (&mut self.buffer[..], &mut self.write_position),
            src,
            bit_len,
        )
    }

    #[inline]
    fn write_bits_with_offset_len(
        &mut self,
        src: &[u8],
        src_bit_offset: usize,
        src_bit_len: usize,
    ) -> Result<(), Error> {
        self.ensure_can_write_additional_bits(src_bit_len);
        BitWrite::write_bits_with_offset_len(
            &mut (&mut self.buffer[..], &mut self.write_position),
            src,
            src_bit_offset,
            src_bit_len,
        )
    }
}

pub struct Bits<'a> {
    slice: &'a [u8],
    pos: usize,
    len: usize,
}

impl<'a> From<&'a [u8]> for Bits<'a> {
    fn from(slice: &'a [u8]) -> Self {
        Self {
            slice,
            pos: 0,
            len: slice.len() * BYTE_LEN,
        }
    }
}

impl<'a> From<(&'a [u8], usize)> for Bits<'a> {
    fn from((slice, len): (&'a [u8], usize)) -> Self {
        debug_assert!(len <= slice.len() * BYTE_LEN);
        Self { slice, pos: 0, len }
    }
}

impl<'a> From<&'a BitBuffer> for Bits<'a> {
    fn from(buffer: &'a BitBuffer) -> Self {
        Self {
            slice: buffer.content(),
            pos: 0,
            len: buffer.bit_len(),
        }
    }
}

impl BitRead for Bits<'_> {
    #[inline]
    fn read_bit(&mut self) -> Result<bool, Error> {
        if self.pos < self.len {
            BitRead::read_bit(&mut (self.slice, &mut self.pos))
        } else {
            Err(ErrorKind::EndOfStream.into())
        }
    }

    #[inline]
    fn read_bits(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        BitRead::read_bits(&mut (self.slice, &mut self.pos), dst)
    }

    #[inline]
    fn read_bits_with_offset(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
    ) -> Result<(), Error> {
        BitRead::read_bits_with_offset(&mut (self.slice, &mut self.pos), dst, dst_bit_offset)
    }

    #[inline]
    fn read_bits_with_len(&mut self, dst: &mut [u8], dst_bit_len: usize) -> Result<(), Error> {
        BitRead::read_bits_with_len(&mut (self.slice, &mut self.pos), dst, dst_bit_len)
    }

    #[inline]
    fn read_bits_with_offset_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
        dst_bit_len: usize,
    ) -> Result<(), Error> {
        BitRead::read_bits_with_offset_len(
            &mut (self.slice, &mut self.pos),
            dst,
            dst_bit_offset,
            dst_bit_len,
        )
    }
}

impl ScopedBitRead for Bits<'_> {
    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    fn set_pos(&mut self, position: usize) -> usize {
        let pos = position.min(self.len);
        self.pos = pos;
        pos
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn set_len(&mut self, len: usize) -> usize {
        let len = len.min(self.slice.len() * BYTE_LEN);
        self.len = len;
        len
    }

    #[inline]
    fn remaining(&self) -> usize {
        self.len - self.pos
    }
}

#[cfg(test)]
#[allow(clippy::identity_op, clippy::inconsistent_digit_grouping)] // this makes various examples easier to understand
pub mod tests {
    use super::*;
    use crate::io::per::unaligned::BitRead;
    use crate::io::per::unaligned::BitWrite;
    use crate::io::per::unaligned::PackedRead;
    use crate::io::per::unaligned::PackedWrite;

    #[test]
    pub fn bit_buffer_write_bit_keeps_correct_order() -> Result<(), Error> {
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

        assert_eq!(buffer.read_bit(), Err(ErrorKind::EndOfStream.into()));

        Ok(())
    }

    #[test]
    fn bit_buffer_bits() -> Result<(), Error> {
        let content = &[0xFF, 0x74, 0xA6, 0x0F];
        let mut buffer = BitBuffer::default();
        buffer.write_bits(content)?;
        assert_eq!(buffer.content(), content);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            let mut content2 = vec![0_u8; content.len()];
            buffer2.read_bits(&mut content2[..])?;
            assert_eq!(&content[..], &content2[..]);
        }

        let mut content2 = vec![0xFF_u8; content.len()];
        buffer.read_bits(&mut content2[..])?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffers_with_offset() -> Result<(), Error> {
        let content = &[0b1111_1111, 0b0111_0100, 0b1010_0110, 0b0000_1111];
        let mut buffer = BitBuffer::default();
        buffer.write_bits_with_offset(content, 7)?;
        assert_eq!(
            buffer.content(),
            &[0b1011_1010, 0b0101_0011, 0b0000_0111, 0b1000_0000]
        );

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            let mut content2 = vec![0xFF_u8; content.len()];
            content2[0] = content[0] & 0b1111_1110; // since we are skipping the first 7 bits
            buffer2.read_bits_with_offset(&mut content2[..], 7)?;
            assert_eq!(&content[..], &content2[..]);
        }

        let mut content2 = vec![0_u8; content.len()];
        content2[0] = content[0] & 0b1111_1110; // since we are skipping the first 7 bits
        buffer.read_bits_with_offset(&mut content2[..], 7)?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffer_bits_with_offset_len() -> Result<(), Error> {
        let content = &[0b1111_1111, 0b0111_0100, 0b1010_0110, 0b0000_1111];
        let mut buffer = BitBuffer::default();
        buffer.write_bits_with_offset_len(content, 7, 12)?;
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
            buffer2.read_bits_with_offset_len(&mut content2[..], 7, 12)?;
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
        buffer.read_bits_with_offset_len(&mut content2[..], 7, 12)?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_0() -> Result<(), Error> {
        const DET: u64 = 0;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(None, None, DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant(None, None)?);
        }

        assert_eq!(DET, buffer.read_length_determinant(None, None)?);

        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_1() -> Result<(), Error> {
        const DET: u64 = 1;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(None, None, DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant(None, None)?);
        }

        assert_eq!(DET, buffer.read_length_determinant(None, None)?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_127() -> Result<(), Error> {
        const DET: u64 = 126;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(None, None, DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant(None, None)?);
        }

        assert_eq!(DET, buffer.read_length_determinant(None, None)?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_128() -> Result<(), Error> {
        const DET: u64 = 128;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(None, None, DET)?;
        // detects that the value is greater than 127, so
        //   10xx_xxxx xxxx_xxxx (header)
        // | --00_0000 1000_0000 (128)
        // =======================
        //   1000_0000 1000_0000
        assert_eq!(buffer.content(), &[0x80 | 0x00, 0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant(None, None)?);
        }

        assert_eq!(DET, buffer.read_length_determinant(None, None)?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_16383() -> Result<(), Error> {
        const DET: u64 = 16383;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(None, None, DET)?;
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
            assert_eq!(DET, buffer2.read_length_determinant(None, None)?);
        }

        assert_eq!(DET, buffer.read_length_determinant(None, None)?);
        Ok(())
    }

    fn check_unconstrained_whole_number(buffer: &mut BitBuffer, int: i64) -> Result<(), Error> {
        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(int, buffer2.read_unconstrained_whole_number()?)
        }

        assert_eq!(int, buffer.read_unconstrained_whole_number()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_neg_12() -> Result<(), Error> {
        const INT: i64 = -12;
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_0() -> Result<(), Error> {
        const INT: i64 = 0;
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_127() -> Result<(), Error> {
        const INT: i64 = 127; // u4::MAX as u64
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_128() -> Result<(), Error> {
        const INT: i64 = 128; // u4::MAX as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        assert_eq!(buffer.content(), &[0x02, 0x00, 0x80]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_255() -> Result<(), Error> {
        const INT: i64 = 255; // u8::MAX as u64
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        assert_eq!(buffer.content(), &[0x02, 0x00, 0xFF]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_256() -> Result<(), Error> {
        const INT: i64 = 256; // u8::MAX as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
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
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_65535() -> Result<(), Error> {
        const INT: i64 = 65_535; // u16::MAX as u64
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        assert_eq!(buffer.content(), &[0x03, 0x00, 0xFF, 0xFF]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_65536() -> Result<(), Error> {
        const INT: i64 = 65_536; // u16::MAX as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        assert_eq!(buffer.content(), &[0x03, 0x01, 0x00, 0x00]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_16777215() -> Result<(), Error> {
        const INT: i64 = 16_777_215; // u24::MAX as u64
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        assert_eq!(buffer.content(), &[0x04, 0x00, 0xFF, 0xFF, 0xFF]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_16777216() -> Result<(), Error> {
        const INT: i64 = 16_777_216; // u24::MAX as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
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
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_4294967295() -> Result<(), Error> {
        const INT: i64 = 4_294_967_295; // u32::MAX as u64
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        assert_eq!(buffer.content(), &[0x05, 0x00, 0xFF, 0xFF, 0xFF, 0xFF]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_4294967296() -> Result<(), Error> {
        const INT: i64 = 4_294_967_296; // u32::MAX as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
        assert_eq!(buffer.content(), &[0x05, 0x01, 0x00, 0x00, 0x00, 0x00]);
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_unconstrained_whole_number_i64_max() -> Result<(), Error> {
        const INT: i64 = i64::MAX;
        let mut buffer = BitBuffer::default();
        buffer.write_unconstrained_whole_number(INT)?;
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
        check_unconstrained_whole_number(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_write_constrained_whole_number_detects_not_in_range_positive_only() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_constrained_whole_number(10, 127, 0),
            Err(ErrorKind::ValueNotInRange(0, 10, 127).into())
        );
        // upper check
        assert_eq!(
            buffer.write_constrained_whole_number(10, 127, 128),
            Err(ErrorKind::ValueNotInRange(128, 10, 127).into())
        );
    }

    #[test]
    fn bit_buffer_write_constrained_whole_number_detects_not_in_range_negative() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_constrained_whole_number(-10, -1, -11),
            Err(ErrorKind::ValueNotInRange(-11, -10, -1).into())
        );
        // upper check
        assert_eq!(
            buffer.write_constrained_whole_number(-10, -1, 0),
            Err(ErrorKind::ValueNotInRange(0, -10, -1).into())
        );
    }

    #[test]
    fn bit_buffer_write_constrained_whole_number_detects_not_in_range_with_negative() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_constrained_whole_number(-10, 1, -11),
            Err(ErrorKind::ValueNotInRange(-11, -10, 1).into())
        );
        // upper check
        assert_eq!(
            buffer.write_constrained_whole_number(-10, 1, 2),
            Err(ErrorKind::ValueNotInRange(2, -10, 1).into())
        );
    }

    fn check_constrained_whole_number(
        buffer: &mut BitBuffer,
        int: i64,
        range: (i64, i64),
    ) -> Result<(), Error> {
        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(
                int,
                buffer2.read_constrained_whole_number(range.0, range.1)?
            );
        }
        assert_eq!(int, buffer.read_constrained_whole_number(range.0, range.1)?);
        Ok(())
    }

    #[test]
    fn bit_buffer_constrained_whole_number_7bits() -> Result<(), Error> {
        const INT: i64 = 10;
        const RANGE: (i64, i64) = (0, 127);
        let mut buffer = BitBuffer::default();
        buffer.write_constrained_whole_number(RANGE.0, RANGE.1, INT)?;
        // [0; 127] are 128 numbers, so they
        // have to fit in 7 bit
        assert_eq!(buffer.content(), &[(INT as u8) << 1]);
        check_constrained_whole_number(&mut buffer, INT, RANGE)?;
        // be sure write_bit writes at the 8th bit
        buffer.write_bit(true)?;
        assert_eq!(buffer.content(), &[(INT as u8) << 1 | 0b0000_0001]);
        Ok(())
    }

    #[test]
    fn bit_buffer_constrained_whole_number_neg() -> Result<(), Error> {
        const INT: i64 = -10;
        const RANGE: (i64, i64) = (-128, 127);
        let mut buffer = BitBuffer::default();
        buffer.write_constrained_whole_number(RANGE.0, RANGE.1, INT)?;
        // [-128; 127] are 255 numbers, so they
        // have to fit in one byte
        assert_eq!(buffer.content(), &[(INT - RANGE.0) as u8]);
        check_constrained_whole_number(&mut buffer, INT, RANGE)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_constrained_whole_number_neg_extended_range() -> Result<(), Error> {
        const INT: i64 = -10;
        const RANGE: (i64, i64) = (-128, 128);
        let mut buffer = BitBuffer::default();
        buffer.write_constrained_whole_number(RANGE.0, RANGE.1, INT)?;
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
        check_constrained_whole_number(&mut buffer, INT, RANGE)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_with_range() -> Result<(), Error> {
        // test scenario from https://github.com/alexvoronov/geonetworking/blob/57a43113aeabc25f005ea17f76409aed148e67b5/camdenm/src/test/java/net/gcdc/camdenm/UperEncoderDecodeTest.java#L169
        const BYTES: &[u8] = &[0x2A, 0x2B, 0x96, 0xFF];
        const RANGE: (u64, u64) = (1, 20);
        let mut buffer = BitBuffer::default();
        buffer.write_octetstring(Some(RANGE.0), Some(RANGE.1), false, BYTES)?;
        assert_eq!(&[0x19, 0x51, 0x5c, 0xb7, 0xf8], &buffer.content());
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_without_range() -> Result<(), Error> {
        const BYTES: &[u8] = &[0x2A, 0x2B, 0x96, 0xFF];
        let mut buffer = BitBuffer::default();
        buffer.write_octetstring(None, None, false, BYTES)?;
        assert_eq!(&[0x04, 0x2a, 0x2b, 0x96, 0xff], &buffer.content());
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_empty() -> Result<(), Error> {
        const BYTES: &[u8] = &[];
        let mut buffer = BitBuffer::default();
        buffer.write_octetstring(None, None, false, BYTES)?;
        assert_eq!(&[0x00], &buffer.content());
        Ok(())
    }

    #[test]
    fn bit_buffer_normally_small_non_negative_whole_number_5() -> Result<(), Error> {
        // example from larmouth-asn1-book, p.296, Figure III-25
        let mut buffer = BitBuffer::default();
        buffer.write_normally_small_non_negative_whole_number(5)?;
        // first 7 bits are relevant
        assert_eq!(&[0b0000_101_0], &buffer.content());
        assert_eq!(5, buffer.read_normally_small_non_negative_whole_number()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_normally_small_non_negative_whole_number_60() -> Result<(), Error> {
        // example from larmouth-asn1-book, p.296, Figure III-25
        let mut buffer = BitBuffer::default();
        buffer.write_normally_small_non_negative_whole_number(60)?;
        // first 7 bits
        assert_eq!(&[0b0111_100_0], &buffer.content());
        assert_eq!(60, buffer.read_normally_small_non_negative_whole_number()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_normally_small_non_negative_whole_number_254() -> Result<(), Error> {
        // example from larmouth-asn1-book, p.296, Figure III-25
        let mut buffer = BitBuffer::default();
        buffer.write_normally_small_non_negative_whole_number(254)?;
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
    fn bit_buffer_write_choice_index_extensible() -> Result<(), Error> {
        fn write_once(index: u64, no_of_default_variants: u64) -> Result<(usize, Vec<u8>), Error> {
            let mut buffer = BitBuffer::default();
            buffer.write_choice_index(no_of_default_variants, true, index)?;
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
    fn bit_buffer_read_choice_index_extensible() -> Result<(), Error> {
        fn read_once(data: &[u8], bits: usize, no_of_variants: u64) -> Result<u64, Error> {
            let mut buffer = BitBuffer::default();
            buffer.write_bits_with_len(data, bits)?;
            buffer.read_choice_index(no_of_variants, true)
        }
        assert_eq!(0, read_once(&[0x00], 2, 2)?);
        assert_eq!(1, read_once(&[0x40], 2, 2)?);
        assert_eq!(2, read_once(&[0x80], 8, 2)?);
        assert_eq!(3, read_once(&[0x81], 8, 2)?);
        Ok(())
    }
}
