use super::BitRead;
use crate::io::per::unaligned::BitWrite;
use crate::io::per::unaligned::BYTE_LEN;
use crate::io::per::{Error, ErrorKind};

impl BitRead for (&[u8], &mut usize) {
    #[inline]
    fn read_bit(&mut self) -> Result<bool, Error> {
        if *self.1 > self.0.len() * BYTE_LEN {
            return Err(ErrorKind::EndOfStream.into());
        }
        let bit = self.0[*self.1 / BYTE_LEN] & (0x80 >> (*self.1 % BYTE_LEN)) != 0;
        *self.1 += 1;
        Ok(bit)
    }

    #[inline]
    fn read_bits(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        self.read_bits_with_offset_len(dst, 0, dst.len() * BYTE_LEN)
    }

    #[inline]
    fn read_bits_with_offset(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
    ) -> Result<(), Error> {
        self.read_bits_with_offset_len(dst, dst_bit_offset, dst.len() * BYTE_LEN - dst_bit_offset)
    }

    #[inline]
    fn read_bits_with_len(&mut self, dst: &mut [u8], dst_bit_len: usize) -> Result<(), Error> {
        self.read_bits_with_offset_len(dst, 0, dst_bit_len)
    }

    #[inline]
    fn read_bits_with_offset_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
        dst_bit_len: usize,
    ) -> Result<(), Error> {
        bit_string_copy_bulked(self.0, *self.1, dst, dst_bit_offset, dst_bit_len)?;
        *self.1 += dst_bit_len;
        Ok(())
    }
}

impl<'a> BitWrite for (&'a mut [u8], &mut usize) {
    #[inline]
    fn write_bit(&mut self, bit: bool) -> Result<(), Error> {
        if *self.1 + 1 > self.0.len() * BYTE_LEN {
            return Err(ErrorKind::EndOfStream.into());
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
    fn write_bits(&mut self, src: &[u8]) -> Result<(), Error> {
        self.write_bits_with_offset(src, 0)
    }

    #[inline]
    fn write_bits_with_offset(&mut self, src: &[u8], src_bit_offset: usize) -> Result<(), Error> {
        self.write_bits_with_offset_len(src, src_bit_offset, src.len() * BYTE_LEN - src_bit_offset)
    }

    #[inline]
    fn write_bits_with_len(&mut self, src: &[u8], bit_len: usize) -> Result<(), Error> {
        self.write_bits_with_offset_len(src, 0, bit_len)
    }

    #[inline]
    fn write_bits_with_offset_len(
        &mut self,
        src: &[u8],
        src_bit_offset: usize,
        src_bit_len: usize,
    ) -> Result<(), Error> {
        bit_string_copy_bulked(src, src_bit_offset, self.0, *self.1, src_bit_len)?;
        *self.1 += src_bit_len;
        Ok(())
    }
}

#[inline]
fn bit_string_copy(
    src: &[u8],
    src_bit_position: usize,
    dst: &mut [u8],
    dst_bit_position: usize,
    len: usize,
) -> Result<(), Error> {
    if dst.len() * BYTE_LEN < dst_bit_position + len {
        return Err(Error::insufficient_space_in_destination_buffer());
    }
    if src.len() * BYTE_LEN < src_bit_position + len {
        return Err(Error::insufficient_data_in_source_buffer());
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

#[inline]
pub(crate) fn bit_string_copy_bulked(
    src: &[u8],
    src_bit_position: usize,
    dst: &mut [u8],
    dst_bit_position: usize,
    len: usize,
) -> Result<(), Error> {
    // chosen by real world tests
    if len <= BYTE_LEN * 2 {
        return bit_string_copy(src, src_bit_position, dst, dst_bit_position, len);
    }

    if dst.len() * BYTE_LEN < dst_bit_position + len {
        return Err(Error::insufficient_space_in_destination_buffer());
    }
    if src.len() * BYTE_LEN < src_bit_position + len {
        return Err(Error::insufficient_data_in_source_buffer());
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
