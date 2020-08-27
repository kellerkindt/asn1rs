use crate::io::per::PackedRead;

pub mod buffer;
pub mod slice;

pub trait BitRead {
    type Error;

    fn read_bit(&mut self) -> Result<bool, Self::Error>;

    fn read_bits(&mut self, dst: &mut [u8]) -> Result<(), Self::Error>;

    fn read_bits_with_offset(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
    ) -> Result<(), Self::Error>;

    fn read_bits_with_len(&mut self, dst: &mut [u8], dst_bit_len: usize)
        -> Result<(), Self::Error>;

    fn read_bits_with_offset_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
        dst_bit_len: usize,
    ) -> Result<(), Self::Error>;
}

impl<T: BitRead> PackedRead for T {
    type Error = T::Error;

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 12
    #[inline]
    fn read_boolean(&mut self) -> Result<bool, Self::Error> {
        self.read_bit()
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.3
    fn read_non_negative_binary_integer(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
    ) -> Result<u64, Self::Error> {
        let range = match (lower_bound, upper_bound) {
            (None, None) => None,
            (lb, ub) => Some((lb.unwrap_or(0), ub.unwrap_or(i64::MAX as u64))),
        };

        if let Some((lower, upper)) = range {
            let range = upper - lower;
            let offset_bits = range.leading_zeros() as usize;
            let mut bytes = [0u8; std::mem::size_of::<u64>()];
            self.read_bits_with_offset(&mut bytes, offset_bits)?;
            Ok(lower + u64::from_be_bytes(bytes))
        } else {
            let length = self.read_length_determinant(None, None)?;
            let mut bytes = [0u8; std::mem::size_of::<u64>()];
            let offset = bytes.len() - length as usize;
            self.read_bits(&mut bytes[offset..])?;
            Ok(u64::from_be_bytes(bytes))
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.4
    fn read_2s_compliment_binary_integer(&mut self, bit_len: u64) -> Result<i64, Self::Error> {
        let mut bytes = [0u8; std::mem::size_of::<i64>()];
        let bits_offset = (bytes.len() * 8) - bit_len as usize;
        self.read_bits_with_offset(&mut bytes, bits_offset)?;
        let byte_offset = bits_offset / 8;
        let bit_offset = bits_offset % 8;
        // check if the most significant bit is set (2er compliment -> negative number)
        if bytes[byte_offset] & (0x80 >> bit_offset) != 0 {
            // negative number, needs to be expanded before converting
            for byte in bytes.iter_mut().take(byte_offset) {
                *byte = 0xFF;
            }
            for i in 0..bit_offset {
                bytes[byte_offset] |= 0x80 >> i;
            }
        }
        Ok(i64::from_be_bytes(bytes))
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.5
    #[inline]
    fn read_constrained_whole_number(
        &mut self,
        lower_bound: i64,
        upper_bound: i64,
    ) -> Result<i64, Self::Error> {
        let range = upper_bound - lower_bound;
        if range > 0 {
            Ok(lower_bound
                + self.read_non_negative_binary_integer(None, Some(range as u64))? as i64)
        } else {
            Ok(lower_bound)
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.6
    fn read_normally_small_non_negative_whole_number(&mut self) -> Result<u64, Self::Error> {
        let greater_or_equal_to_64 = self.read_boolean()?;
        if greater_or_equal_to_64 {
            // 11.6.2: self.read_semi_constrained_whole_number(0)
            // 11.7.4: self.read_non_negative_binary_integer(0, MAX) + lb  | lb=0=>MIN for unsigned
            self.read_non_negative_binary_integer(None, None)
        } else {
            // 11.6.1
            self.read_non_negative_binary_integer(None, Some(63))
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.7
    #[inline]
    fn read_semi_constrained_whole_number(&mut self, lower_bound: i64) -> Result<i64, Self::Error> {
        let n = self.read_non_negative_binary_integer(None, None)?;
        Ok((n as i64) + lower_bound)
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.8
    #[inline]
    fn read_unconstrained_whole_number(&mut self) -> Result<i64, Self::Error> {
        let octet_len = self.read_length_determinant(None, None)?;
        self.read_2s_compliment_binary_integer(octet_len * 8)
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.9.3
    #[inline]
    fn read_normally_small_length(&mut self) -> Result<u64, Self::Error> {
        self.read_normally_small_non_negative_whole_number()
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.9.4
    fn read_length_determinant(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
    ) -> Result<u64, Self::Error> {
        let lower_bound_unwrapped = lower_bound.unwrap_or(0);
        let upper_bound_unwrapped = upper_bound.unwrap_or_else(|| i64::MAX as u64);

        if (lower_bound.is_some() || upper_bound.is_some()) && upper_bound_unwrapped >= 64 * 1024 {
            // 11.9.4.2
            if lower_bound == upper_bound {
                Ok(lower_bound_unwrapped)
            } else {
                Ok(lower_bound_unwrapped
                    + self.read_non_negative_binary_integer(lower_bound, upper_bound)?)
            }
        } else if upper_bound.is_some() && upper_bound_unwrapped <= 64 * 1024 {
            // 11.9.4.1 -> 11.9.3.4 -> 11.6.1
            Ok(self.read_non_negative_binary_integer(lower_bound, upper_bound)?)
        } else {
            // 11.9.4.1 -> 11.9.3.5
            if !self.read_bit()? {
                // 11.9.3.6: less than or equal to 127
                self.read_non_negative_binary_integer(None, Some(127))
            } else if !self.read_bit()? {
                // 11.9.3.7: greater than 127 and less than or equal to 16K
                self.read_non_negative_binary_integer(None, Some(16 * 1024 - 1))
            } else {
                // 11.9.3.8: chunks of 16k multiples
                let mut multiple = [0u8; 1];
                self.read_bits_with_offset(&mut multiple[..], 2)?;
                Ok(16 * 1024 * u64::from(multiple[0].max(4 /* 11.9.3.8, NOTE */)))
            }
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 16
    fn read_bitstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
    ) -> Result<(Vec<u8>, u64), Self::Error> {
        // let lower_bound = lower_bound_size.unwrap_or_default();
        let upper_bound = upper_bound_size.unwrap_or_else(|| i64::MAX as u64);

        let (mut bit_len, fragmentation_possible) = if extensible && self.read_bit()? {
            // 16.6
            // self.read_semi_constrained_whole_number(0)
            // self.read_non_negative_binary_integer(0, MAX) + lb  | lb=0=>MIN for unsigned
            (self.read_length_determinant(None, None)?, true)
        }
        /*else if lower_bound_size.is_some()
            && lower_bound_size == upper_bound_size
            && upper_bound <= 16
        {
            // 16.9
            (upper_bound, false)
        }*/
        else if lower_bound_size.is_some()
            && lower_bound_size == upper_bound_size
            && upper_bound < 64 * 1024
        {
            // 16.10
            (upper_bound, false)
        } else {
            // 16.11
            (
                self.read_length_determinant(lower_bound_size, upper_bound_size)?,
                true,
            )
        };

        let mut byte_len = (bit_len + 7) / 8;
        let mut buffer = vec![0u8; byte_len as usize];
        self.read_bits_with_len(&mut buffer[..], bit_len as usize)?;

        // fragmentation?
        if fragmentation_possible && bit_len >= 16 * 1024 {
            loop {
                let ext_bit_len = self.read_length_determinant(None, None)?;
                let ext_byte_len = byte_len - ((bit_len + ext_bit_len) + 7) / 8;
                buffer.extend(core::iter::repeat(0x00).take(ext_byte_len as usize));
                self.read_bits_with_offset_len(
                    &mut buffer[..],
                    bit_len as usize,
                    ext_bit_len as usize,
                )?;

                bit_len += ext_bit_len;
                byte_len += ext_bit_len;

                if ext_bit_len < 16 * 1024 {
                    break;
                }
            }
        }

        Ok((buffer, bit_len))
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 17
    fn read_octetstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
    ) -> Result<Vec<u8>, Self::Error> {
        // let lower_bound = lower_bound_size.unwrap_or_default();
        let upper_bound = upper_bound_size.unwrap_or_else(|| i64::MAX as u64);

        let (mut byte_len, fragmentation_possible) = if extensible && self.read_bit()? {
            // 17.3
            // self.read_semi_constrained_whole_number(0)
            // self.read_non_negative_binary_integer(0, MAX) + lb  | lb=0=>MIN for unsigned
            (self.read_length_determinant(None, None)?, true)
        } else if upper_bound == 0 {
            // 17.5
            return Ok(Vec::default());
        }
        /* else if lower_bound_size.is_some()
            && lower_bound_size == upper_bound_size
            && upper_bound <= 2
        {
            // 17.6
            (upper_bound, false)
        }*/
        else if lower_bound_size.is_some()
            && lower_bound_size == upper_bound_size
            && upper_bound < 64 * 1024
        {
            // 17.7
            (upper_bound, false)
        } else {
            // 17.8
            (
                self.read_length_determinant(lower_bound_size, upper_bound_size)?,
                true,
            )
        };

        let mut buffer = vec![0u8; byte_len as usize];
        self.read_bits(&mut buffer[..])?;

        // fragmentation?
        if fragmentation_possible && byte_len >= 16 * 1024 {
            loop {
                let ext_byte_len = self.read_length_determinant(None, None)?;
                buffer.extend(core::iter::repeat(0u8).take(ext_byte_len as usize));
                self.read_bits(&mut buffer[byte_len as usize..])?;
                byte_len += ext_byte_len;

                if ext_byte_len < 16 * 1024 {
                    break;
                }
            }
        }

        Ok(buffer)
    }
}

pub trait BitWrite {
    type Error;

    fn write_bit(&mut self, bit: bool) -> Result<(), Self::Error>;

    fn write_bits(&mut self, src: &[u8]) -> Result<(), Self::Error>;

    fn write_bits_with_offset(
        &mut self,
        src: &[u8],
        src_bit_offset: usize,
    ) -> Result<(), Self::Error>;

    fn write_bits_with_len(&mut self, src: &[u8], bit_len: usize) -> Result<(), Self::Error>;

    fn write_bits_with_offset_len(
        &mut self,
        src: &[u8],
        src_bit_offset: usize,
        src_bit_len: usize,
    ) -> Result<(), Self::Error>;
}
