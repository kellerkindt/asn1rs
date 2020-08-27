use super::BitRead;
use crate::io::buffer::bit_string_copy_bulked;
use crate::io::per::unaligned::BitWrite;
pub(crate) use crate::io::per::Error;
pub(crate) use crate::io::per::BYTE_LEN;

impl BitRead for (&[u8], &mut usize) {
    type Error = Error;

    #[inline]
    fn read_bit(&mut self) -> Result<bool, Self::Error> {
        if *self.1 > self.0.len() * BYTE_LEN {
            return Err(Error::EndOfStream);
        }
        let bit = self.0[*self.1 / BYTE_LEN] & (0x80 >> (*self.1 % BYTE_LEN)) != 0;
        *self.1 += 1;
        Ok(bit)
    }

    #[inline]
    fn read_bits(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        self.read_bits_with_offset_len(dst, 0, dst.len() * BYTE_LEN)
    }

    #[inline]
    fn read_bits_with_offset(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
    ) -> Result<(), Self::Error> {
        self.read_bits_with_offset_len(dst, dst_bit_offset, dst.len() * BYTE_LEN - dst_bit_offset)
    }

    #[inline]
    fn read_bits_with_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_len: usize,
    ) -> Result<(), Self::Error> {
        self.read_bits_with_offset_len(dst, 0, dst_bit_len)
    }

    #[inline]
    fn read_bits_with_offset_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
        dst_bit_len: usize,
    ) -> Result<(), Self::Error> {
        bit_string_copy_bulked(&self.0[..], *self.1, dst, dst_bit_offset, dst_bit_len)?;
        *self.1 += dst_bit_len;
        Ok(())
    }
}

impl<'a> BitWrite for (&'a mut [u8], &mut usize) {
    type Error = Error;

    #[inline]
    fn write_bit(&mut self, bit: bool) -> Result<(), Self::Error> {
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

    #[inline]
    fn write_bits(&mut self, src: &[u8]) -> Result<(), Self::Error> {
        self.write_bits_with_offset(src, 0)
    }

    #[inline]
    fn write_bits_with_offset(
        &mut self,
        src: &[u8],
        src_bit_offset: usize,
    ) -> Result<(), Self::Error> {
        self.write_bits_with_offset_len(src, src_bit_offset, src.len() * BYTE_LEN - src_bit_offset)
    }

    #[inline]
    fn write_bits_with_len(&mut self, src: &[u8], bit_len: usize) -> Result<(), Self::Error> {
        self.write_bits_with_offset_len(src, 0, bit_len)
    }

    #[inline]
    fn write_bits_with_offset_len(
        &mut self,
        src: &[u8],
        src_bit_offset: usize,
        src_bit_len: usize,
    ) -> Result<(), Self::Error> {
        bit_string_copy_bulked(src, src_bit_offset, &mut self.0[..], *self.1, src_bit_len)?;
        *self.1 += src_bit_len;
        Ok(())
    }
}
