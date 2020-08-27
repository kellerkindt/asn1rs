use super::*;
use crate::io::buffer::BitBuffer;
use crate::io::per::BYTE_LEN;

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

impl BitWrite for BitBuffer {
    type Error = super::slice::Error;

    #[inline]
    fn write_bit(&mut self, bit: bool) -> Result<(), Self::Error> {
        self.ensure_can_write_additional_bits(1);
        BitWrite::write_bit(&mut (&mut self.buffer[..], &mut self.write_position), bit)
    }

    #[inline]
    fn write_bits(&mut self, src: &[u8]) -> Result<(), Self::Error> {
        self.ensure_can_write_additional_bits(src.len() * BYTE_LEN);
        BitWrite::write_bits(&mut (&mut self.buffer[..], &mut self.write_position), src)
    }

    #[inline]
    fn write_bits_with_offset(
        &mut self,
        src: &[u8],
        src_bit_offset: usize,
    ) -> Result<(), Self::Error> {
        self.ensure_can_write_additional_bits(src.len() * BYTE_LEN - src_bit_offset);
        BitWrite::write_bits_with_offset(
            &mut (&mut self.buffer[..], &mut self.write_position),
            src,
            src_bit_offset,
        )
    }

    #[inline]
    fn write_bits_with_len(&mut self, src: &[u8], bit_len: usize) -> Result<(), Self::Error> {
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
    ) -> Result<(), Self::Error> {
        self.ensure_can_write_additional_bits(src_bit_len);
        BitWrite::write_bits_with_offset_len(
            &mut (&mut self.buffer[..], &mut self.write_position),
            src,
            src_bit_offset,
            src_bit_len,
        )
    }
}
