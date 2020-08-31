use super::*;
use crate::io::per::Error;

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

impl BitRead for BitBuffer {
    #[inline]
    fn read_bit(&mut self) -> Result<bool, Error> {
        if self.read_position < self.write_position {
            BitRead::read_bit(&mut (&self.buffer[..], &mut self.read_position))
        } else {
            Err(Error::EndOfStream)
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
