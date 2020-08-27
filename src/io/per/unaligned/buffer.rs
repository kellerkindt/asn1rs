use super::*;
use crate::io::buffer::BitBuffer;

impl BitRead for BitBuffer {
    type Error = super::slice::Error;

    #[inline]
    fn read_bit(&mut self) -> Result<bool, Self::Error> {
        if self.read_position < self.write_position {
            BitRead::read_bit(&mut (&self.buffer[..], &mut self.read_position))
        } else {
            Err(Self::Error::EndOfStream)
        }
    }

    #[inline]
    fn read_bits(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        BitRead::read_bits(&mut (&self.buffer[..], &mut self.read_position), dst)
    }

    #[inline]
    fn read_bits_with_offset(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
    ) -> Result<(), Self::Error> {
        BitRead::read_bits_with_offset(
            &mut (&self.buffer[..], &mut self.read_position),
            dst,
            dst_bit_offset,
        )
    }

    #[inline]
    fn read_bits_with_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_len: usize,
    ) -> Result<(), Self::Error> {
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
    ) -> Result<(), Self::Error> {
        BitRead::read_bits_with_offset_len(
            &mut (&self.buffer[..], &mut self.read_position),
            dst,
            dst_bit_offset,
            dst_bit_len,
        )
    }
}
