use crate::io::per::Error;
use crate::io::per::{PackedRead, PackedWrite};

pub mod buffer;
pub mod slice;

pub const BYTE_LEN: usize = 8;

const FRAGMENT_SIZE: u64 = 16 * 1024;
const MAX_FRAGMENTS: u64 = 4  /* 11.9.3.8, NOTE */ ;
const MIN_FRAGMENT_SIZE: u64 = FRAGMENT_SIZE;
const MAX_FRAGMENT_SIZE: u64 = FRAGMENT_SIZE * MAX_FRAGMENTS;

const LENGTH_127: u64 = 127;
const LENGTH_16K: u64 = 16 * 1024;
const LENGTH_64K: u64 = 64 * 1024;

pub trait BitRead {
    fn read_bit(&mut self) -> Result<bool, Error>;

    fn read_bits(&mut self, dst: &mut [u8]) -> Result<(), Error>;

    fn read_bits_with_offset(&mut self, dst: &mut [u8], dst_bit_offset: usize)
        -> Result<(), Error>;

    fn read_bits_with_len(&mut self, dst: &mut [u8], dst_bit_len: usize) -> Result<(), Error>;

    fn read_bits_with_offset_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
        dst_bit_len: usize,
    ) -> Result<(), Error>;
}

impl<T: BitRead> PackedRead for T {
    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 12
    #[inline]
    fn read_boolean(&mut self) -> Result<bool, Error> {
        self.read_bit()
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.3
    #[inline]
    fn read_non_negative_binary_integer(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
    ) -> Result<u64, Error> {
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
    #[inline]
    fn read_2s_compliment_binary_integer(&mut self, bit_len: u64) -> Result<i64, Error> {
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
    ) -> Result<i64, Error> {
        let range = upper_bound - lower_bound;
        if range > 0 {
            Ok(lower_bound
                + self.read_non_negative_binary_integer(None, Some(range as u64))? as i64)
        } else {
            Ok(lower_bound)
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.6
    #[inline]
    fn read_normally_small_non_negative_whole_number(&mut self) -> Result<u64, Error> {
        let greater_or_equal_to_64 = self.read_bit()?;
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
    fn read_semi_constrained_whole_number(&mut self, lower_bound: i64) -> Result<i64, Error> {
        let n = self.read_non_negative_binary_integer(None, None)?;
        Ok((n as i64) + lower_bound)
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.8
    #[inline]
    fn read_unconstrained_whole_number(&mut self) -> Result<i64, Error> {
        let octet_len = self.read_length_determinant(None, None)?;
        self.read_2s_compliment_binary_integer(octet_len * 8)
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.9.3
    #[inline]
    fn read_normally_small_length(&mut self) -> Result<u64, Error> {
        self.read_normally_small_non_negative_whole_number()
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.9.4
    #[inline]
    fn read_length_determinant(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
    ) -> Result<u64, Error> {
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
            self.read_non_negative_binary_integer(lower_bound, upper_bound)
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
    #[inline]
    #[allow(clippy::suspicious_else_formatting)] // for 16.9 else-if comment block
    fn read_bitstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
    ) -> Result<(Vec<u8>, u64), Error> {
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
    #[inline]
    #[allow(clippy::suspicious_else_formatting)] // for 17.6 else-if comment block
    fn read_octetstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
    ) -> Result<Vec<u8>, Error> {
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

    #[inline]
    fn read_choice_index(&mut self, std_variants: u64, extensible: bool) -> Result<u64, Error> {
        self.read_enumeration_index(std_variants, extensible)
    }

    #[inline]
    fn read_enumeration_index(
        &mut self,
        std_variants: u64,
        extensible: bool,
    ) -> Result<u64, Error> {
        if extensible && self.read_bit()? {
            Ok(self.read_normally_small_length()? + std_variants)
        } else {
            self.read_non_negative_binary_integer(None, Some(std_variants - 1))
        }
    }
}

pub trait BitWrite {
    fn write_bit(&mut self, bit: bool) -> Result<(), Error>;

    fn write_bits(&mut self, src: &[u8]) -> Result<(), Error>;

    fn write_bits_with_offset(&mut self, src: &[u8], src_bit_offset: usize) -> Result<(), Error>;

    fn write_bits_with_len(&mut self, src: &[u8], bit_len: usize) -> Result<(), Error>;

    fn write_bits_with_offset_len(
        &mut self,
        src: &[u8],
        src_bit_offset: usize,
        src_bit_len: usize,
    ) -> Result<(), Error>;
}

impl<T: BitWrite> PackedWrite for T {
    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 12
    #[inline]
    fn write_boolean(&mut self, boolean: bool) -> Result<(), Error> {
        self.write_bit(boolean)
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.3
    #[inline]
    fn write_non_negative_binary_integer(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
        value: u64,
    ) -> Result<(), Error> {
        let range = match (lower_bound, upper_bound) {
            (None, None) => None,
            (lb, ub) => Some((lb.unwrap_or(0), ub.unwrap_or(i64::MAX as u64))),
        };

        if let Some((lower, upper)) = range {
            let range = upper - lower;
            let offset_bits = range.leading_zeros() as usize;
            let bytes = (value - lower).to_be_bytes();
            self.write_bits_with_offset(&bytes[..], offset_bits)?;
            Ok(())
        } else {
            let offset = value.leading_zeros() as u64 / 8;
            let len = std::mem::size_of::<u64>() as u64 - offset;
            let bytes = value.to_be_bytes();
            self.write_length_determinant(None, None, len)?;
            self.write_bits(&bytes[offset as usize..])
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.4
    #[inline]
    fn write_2s_compliment_binary_integer(
        &mut self,
        bit_len: u64,
        value: i64,
    ) -> Result<(), Error> {
        let bytes = value.to_be_bytes();
        let bits_offset = (bytes.len() * 8) - bit_len as usize;
        self.write_bits_with_offset(&bytes[..], bits_offset)
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.5
    #[inline]
    fn write_constrained_whole_number(
        &mut self,
        lower_bound: i64,
        upper_bound: i64,
        value: i64,
    ) -> Result<(), Error> {
        let range = upper_bound - lower_bound;
        if range > 0 {
            if value < lower_bound || value > upper_bound {
                Err(Error::ValueNotInRange(value, lower_bound, upper_bound))
            } else {
                self.write_non_negative_binary_integer(
                    None,
                    Some(range as u64),
                    (value - lower_bound) as u64,
                )
            }
        } else {
            Ok(())
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.6
    #[inline]
    fn write_normally_small_non_negative_whole_number(&mut self, value: u64) -> Result<(), Error> {
        let greater_or_equal_to_64 = value >= 64;
        self.write_bit(greater_or_equal_to_64)?;
        if greater_or_equal_to_64 {
            // 11.6.2: self.write_semi_constrained_whole_number(0)
            // 11.7.4: self.write_non_negative_binary_integer(0, MAX) + lb  | lb=0=>MIN for unsigned
            self.write_non_negative_binary_integer(None, None, value)
        } else {
            // 11.6.1
            self.write_non_negative_binary_integer(None, Some(63), value)
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.7
    #[inline]
    fn write_semi_constrained_whole_number(
        &mut self,
        lower_bound: i64,
        value: i64,
    ) -> Result<(), Error> {
        if value < lower_bound {
            Err(Error::ValueNotInRange(value, lower_bound, i64::MAX))
        } else {
            self.write_non_negative_binary_integer(None, None, (value - lower_bound) as u64)
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.8
    #[inline]
    fn write_unconstrained_whole_number(&mut self, value: i64) -> Result<(), Error> {
        let prefix_len = if value.is_negative() {
            value.leading_ones().saturating_sub(1)
        } else {
            value.leading_zeros().saturating_sub(1)
        } as u64
            / 8;
        let octet_len = core::mem::size_of::<i64>() as u64 - prefix_len;
        self.write_length_determinant(None, None, octet_len)?;
        self.write_2s_compliment_binary_integer(octet_len * 8, value)
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.9.3
    #[inline]
    fn write_normally_small_length(&mut self, value: u64) -> Result<(), Error> {
        self.write_normally_small_non_negative_whole_number(value)
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 11.9.4
    #[inline]
    fn write_length_determinant(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
        value: u64,
    ) -> Result<(), Error> {
        let lower_bound_unwrapped = lower_bound.unwrap_or(0);
        let upper_bound_unwrapped = upper_bound.unwrap_or_else(|| i64::MAX as u64);

        if (lower_bound.is_some() || upper_bound.is_some()) && upper_bound_unwrapped >= LENGTH_64K {
            // 11.9.4.2
            if lower_bound == upper_bound {
                Ok(())
            } else if value < lower_bound_unwrapped {
                Err(Error::ValueNotInRange(
                    value as i64,
                    lower_bound_unwrapped as i64,
                    upper_bound_unwrapped as i64,
                ))
            } else {
                self.write_non_negative_binary_integer(
                    lower_bound,
                    upper_bound,
                    value - lower_bound_unwrapped,
                )
            }
        } else if upper_bound.is_some() && upper_bound_unwrapped <= LENGTH_64K {
            // 11.9.4.1 -> 11.9.3.4 -> 11.6.1
            self.write_non_negative_binary_integer(lower_bound, upper_bound, value)
        } else {
            // 11.9.4.1 -> 11.9.3.5
            if value <= LENGTH_127 {
                // 11.9.3.6: less than or equal to 127
                self.write_bit(false)?;
                self.write_non_negative_binary_integer(None, Some(LENGTH_127), value)
            } else if value <= LENGTH_16K {
                // 11.9.3.7: greater than 127 and less than or equal to 16K
                self.write_bit(true)?;
                self.write_bit(false)?;
                self.write_non_negative_binary_integer(None, Some(LENGTH_16K - 1), value)
            } else {
                // 11.9.3.8: chunks of 16k multiples
                self.write_bit(true)?;
                self.write_bit(true)?;
                let multiple = (value / LENGTH_16K).max(MAX_FRAGMENTS);
                let multiple = [multiple as u8];
                self.write_bits_with_offset(&multiple[..], 2)?;
                Ok(())
            }
        }
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 16
    #[inline]
    #[allow(clippy::suspicious_else_formatting)] // for 16.9 else-if comment block
    fn write_bitstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
        src: &[u8],
        offset: u64,
        len: u64,
    ) -> Result<(), Error> {
        let lower_bound = lower_bound_size.unwrap_or_default();
        let upper_bound = upper_bound_size.unwrap_or_else(|| i64::MAX as u64);
        let length = len;
        let fragmented = length > MAX_FRAGMENT_SIZE;
        let out_of_range = length < lower_bound || length > upper_bound;

        if extensible {
            self.write_bit(out_of_range)?;
        }

        if out_of_range {
            if extensible {
                // 16.6
                // self.read_semi_constrained_whole_number(0)
                // self.read_non_negative_binary_integer(0, MAX) + lb  | lb=0=>MIN for unsigned
                self.write_length_determinant(None, None, length)?;
            } else {
                return Err(Error::SizeNotInRange(length, lower_bound, upper_bound));
            }
        }
        /*else if lower_bound_size.is_some()
            && lower_bound_size == upper_bound_size
            && upper_bound <= 16
        {
            // 16.9
        }*/
        else if lower_bound_size.is_some()
            && lower_bound_size == upper_bound_size
            && upper_bound < LENGTH_64K
        {
            // 16.10
        } else {
            // 16.11
            self.write_length_determinant(lower_bound_size, upper_bound_size, length)?;
        }

        self.write_bits_with_offset_len(
            src,
            offset as usize,
            MAX_FRAGMENT_SIZE.min(length) as usize,
        )?;

        if fragmented {
            let mut written_bits = MAX_FRAGMENT_SIZE;
            loop {
                let fragment_size = (length - written_bits).min(MAX_FRAGMENT_SIZE);
                let fragment_size = fragment_size - (fragment_size % MIN_FRAGMENT_SIZE);
                self.write_length_determinant(None, None, fragment_size)?;
                self.write_bits_with_offset_len(
                    src,
                    (offset + written_bits) as usize,
                    fragment_size as usize,
                )?;
                written_bits += fragment_size;

                if fragment_size < MIN_FRAGMENT_SIZE {
                    break;
                }
            }
        }

        Ok(())
    }

    /// ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 17
    #[inline]
    #[allow(clippy::suspicious_else_formatting)] // for 17.6 else-if comment block
    fn write_octetstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
        src: &[u8],
    ) -> Result<(), Error> {
        let lower_bound = lower_bound_size.unwrap_or_default();
        let upper_bound = upper_bound_size.unwrap_or_else(|| i64::MAX as u64);
        let length = src.len() as u64;
        let fragmented = length > MAX_FRAGMENT_SIZE;
        let out_of_range = length < lower_bound || length > upper_bound;

        if extensible {
            self.write_bit(out_of_range)?;
        }

        if out_of_range {
            if extensible {
                // 17.3
                // self.read_semi_constrained_whole_number(0)
                // self.read_non_negative_binary_integer(0, MAX) + lb  | lb=0=>MIN for unsigned
                self.write_length_determinant(None, None, length)?;
            } else {
                return Err(Error::SizeNotInRange(length, lower_bound, upper_bound));
            }
        } else if upper_bound == 0 {
            // 17.5
            return Ok(());
        }
        /*else if lower_bound_size.is_some()
            && lower_bound_size == upper_bound_size
            && upper_bound <= 2
        {
            // 17.6
        }*/
        else if lower_bound_size.is_some()
            && lower_bound_size == upper_bound_size
            && upper_bound < LENGTH_64K
        {
            // 17.7
        } else {
            // 17.8
            self.write_length_determinant(lower_bound_size, upper_bound_size, length)?;
        }

        self.write_bits(&src[..MAX_FRAGMENT_SIZE.min(length) as usize])?;

        if fragmented {
            let mut written_bytes = MAX_FRAGMENT_SIZE;
            loop {
                let fragment_size = (length - written_bytes).min(MAX_FRAGMENT_SIZE);
                let fragment_size = fragment_size - (fragment_size % MIN_FRAGMENT_SIZE);
                self.write_length_determinant(None, None, fragment_size)?;
                self.write_bits(
                    &src[written_bytes as usize..(written_bytes + fragment_size) as usize],
                )?;
                written_bytes += fragment_size;

                if fragment_size < MIN_FRAGMENT_SIZE {
                    break;
                }
            }
        }

        Ok(())
    }

    #[inline]
    fn write_choice_index(
        &mut self,
        std_variants: u64,
        extensible: bool,
        index: u64,
    ) -> Result<(), Error> {
        self.write_enumeration_index(std_variants, extensible, index)
    }

    #[inline]
    fn write_enumeration_index(
        &mut self,
        std_variants: u64,
        extensible: bool,
        index: u64,
    ) -> Result<(), Error> {
        let out_of_range = index >= std_variants;
        if extensible {
            self.write_bit(out_of_range)?;
        }

        if out_of_range {
            if extensible {
                self.write_normally_small_length(index - std_variants)
            } else {
                Err(Error::InvalidChoiceIndex(index, std_variants))
            }
        } else {
            self.write_non_negative_binary_integer(None, Some(std_variants - 1), index)
        }
    }
}
