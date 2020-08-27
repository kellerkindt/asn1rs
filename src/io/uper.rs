use crate::io::buffer::BitBuffer;
use crate::io::per::unaligned::{BitRead, BitWrite};
use crate::io::per::{PackedRead, PackedWrite};
use byteorder::ByteOrder;
use byteorder::NetworkEndian;

pub const BYTE_LEN: usize = 8;

pub const UPER_LENGTH_DET_L1: i64 = 127;
pub const UPER_LENGTH_DET_L2: i64 = 16383;
// pub const UPER_LENGTH_DET_L3: i64 = 49151;
// pub const UPER_LENGTH_DET_L4: i64 = 65535;

#[derive(Debug, PartialOrd, PartialEq)]
pub enum Error {
    InvalidUtf8String,
    UnsupportedOperation(String),
    InsufficientSpaceInDestinationBuffer,
    InsufficientDataInSourceBuffer,
    InvalidChoiceIndex(u64, u64),
    InvalidExtensionConstellation(bool, bool),
    ValueNotInRange(i64, i64, i64),
    ValueExceedsMaxInt,
    ValueIsNegativeButExpectedUnsigned(i64),
    SizeNotInRange(u64, u64, u64),
    OptFlagsExhausted,
    EndOfStream,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidUtf8String => {
                write!(f, "The underlying dataset is not a valid UTF8-String")
            }
            Error::UnsupportedOperation(o) => write!(f, "The operation is not supported: {}", o),
            Error::InsufficientSpaceInDestinationBuffer => write!(
                f,
                "There is insufficient space in the destination buffer for this operation"
            ),
            Error::InsufficientDataInSourceBuffer => write!(
                f,
                "There is insufficient data in the source buffer for this operation"
            ),
            Error::InvalidChoiceIndex(index, variant_count) => write!(
                f,
                "Unexpected choice-index {} with variant count {}",
                index, variant_count
            ),
            Error::InvalidExtensionConstellation(expects, has) => write!(
                f,
                "Unexpected extension constellation, expected: {}, read: {}",
                expects, has
            ),
            Error::ValueNotInRange(value, min, max) => write!(
                f,
                "The value {} is not within the inclusive range of {} and {}",
                value, min, max
            ),
            Error::ValueExceedsMaxInt => {
                write!(f, "The value exceeds the maximum supported integer size",)
            }
            Error::ValueIsNegativeButExpectedUnsigned(value) => write!(
                f,
                "The value {} is negative, but expected an unsigned/positive value",
                value
            ),
            Error::SizeNotInRange(size, min, max) => write!(
                f,
                "The size {} is not within the inclusive range of {} and {}",
                size, min, max
            ),
            Error::OptFlagsExhausted => write!(f, "All optional flags have already been exhausted"),
            Error::EndOfStream => write!(
                f,
                "Can no longer read or write any bytes from the underlying dataset"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "encoding or decoding UPER failed"
    }
}

#[deprecated(note = "Use the UperReader/-Writer with the Read-/Writable interface instead")]
#[cfg(feature = "legacy-uper-codegen")]
pub trait Uper {
    fn read_uper(reader: &mut dyn Reader) -> Result<Self, Error>
    where
        Self: Sized;

    fn write_uper(&self, writer: &mut dyn Writer) -> Result<(), Error>;
}

#[deprecated(note = "Use the UperReader/-Writer with the Read-/Writable interface instead")]
pub trait Reader {
    /// Sub-strings larger than 16k are not supported
    fn read_substring_with_length_determinant_prefix(&mut self) -> Result<BitBuffer, Error> {
        let byte_len = self.read_length_determinant()?;
        let bit_len = byte_len * BYTE_LEN;
        let mut bytes = vec![0x00_u8; byte_len];
        self.read_bit_string(&mut bytes[..], 0, bit_len)?;
        Ok(BitBuffer::from_bits(bytes, bit_len))
    }

    fn read_utf8_string(&mut self) -> Result<String, Error>;
    /* {
        let len = self.read_length_determinant()?;
        let mut buffer = vec![0_u8; len];
        self.read_bit_string_till_end(&mut buffer[..len], 0)?;
        if let Ok(string) = String::from_utf8(buffer) {
            Ok(string)
        } else {
            Err(Error::InvalidUtf8String)
        }
    }*/

    fn read_choice_index_extensible(&mut self, no_of_default_variants: u64) -> Result<u64, Error>;
    /*{
        if self.read_bit()? {
            Ok((self.read_int_normally_small()? + no_of_default_variants) as u64)
        } else {
            self.read_choice_index(no_of_default_variants)
        }
    }*/

    fn read_choice_index(&mut self, no_of_default_variants: u64) -> Result<u64, Error>;
    /*{
        Ok(self.read_int((0, no_of_default_variants as i64 - 1))? as u64)
    }*/

    /// Range is inclusive
    fn read_int(&mut self, range: (i64, i64)) -> Result<i64, Error>;
    /* {
        let (lower, upper) = range;
        let leading_zeros = ((upper - lower) as u64).leading_zeros();

        let mut buffer = [0_u8; 8];
        let buffer_bits = buffer.len() * BYTE_LEN as usize;
        debug_assert!(buffer_bits == 64);
        self.read_bit_string_till_end(&mut buffer[..], leading_zeros as usize)?;
        let value = NetworkEndian::read_u64(&buffer[..]) as i64;
        Ok(value + lower)
    }*/

    fn read_int_normally_small(&mut self) -> Result<u64, Error>;
    /*{
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
    }*/

    fn read_int_max_signed(&mut self) -> Result<i64, Error>;
    /*{
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
    }*/

    fn read_int_max_unsigned(&mut self) -> Result<u64, Error>;
    /* {
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
    }*/

    fn read_bit_string(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error>;
    /* {
        if buffer.len() * BYTE_LEN < bit_offset || buffer.len() * BYTE_LEN < bit_offset + bit_length
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
    }*/

    fn read_octet_string(&mut self, length_range: Option<(i64, i64)>) -> Result<Vec<u8>, Error>;
    /*{
        let len = if let Some((min, max)) = length_range {
            self.read_int((min, max))? as usize
        } else {
            self.read_length_determinant()?
        };
        let mut vec = vec![0_u8; len];
        self.read_bit_string_till_end(&mut vec[..], 0)?;
        Ok(vec)
    }*/

    fn read_bit_string_till_end(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
    ) -> Result<(), Error>;
    /* {
        let len = (buffer.len() * BYTE_LEN) - bit_offset;
        self.read_bit_string(buffer, bit_offset, len)
    }*/

    #[allow(clippy::if_not_else)]
    fn read_length_determinant(&mut self) -> Result<usize, Error>;
    /*{
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
    }*/

    fn read_bit(&mut self) -> Result<bool, Error>;
}

#[allow(deprecated)]
impl<T: BitRead<Error = Error> + PackedRead<Error = Error>> Reader for T {
    #[inline]
    fn read_utf8_string(&mut self) -> Result<String, Error> {
        let octets = <T as PackedRead>::read_octetstring(self, None, None, false)?;
        String::from_utf8(octets).map_err(|_| Error::InvalidUtf8String)
    }

    #[inline]
    fn read_choice_index_extensible(&mut self, no_of_default_variants: u64) -> Result<u64, Error> {
        <T as PackedRead>::read_choice_index(self, no_of_default_variants, true)
    }

    #[inline]
    fn read_choice_index(&mut self, no_of_default_variants: u64) -> Result<u64, Error> {
        <T as PackedRead>::read_choice_index(self, no_of_default_variants, false)
    }

    #[inline]
    fn read_int(&mut self, (lower_bound, upper_bound): (i64, i64)) -> Result<i64, Error> {
        <T as PackedRead>::read_constrained_whole_number(self, lower_bound, upper_bound)
    }

    #[inline]
    fn read_int_normally_small(&mut self) -> Result<u64, Error> {
        <T as PackedRead>::read_normally_small_non_negative_whole_number(self)
    }

    #[inline]
    fn read_int_max_signed(&mut self) -> Result<i64, Error> {
        <T as PackedRead>::read_unconstrained_whole_number(self)
    }

    #[inline]
    fn read_int_max_unsigned(&mut self) -> Result<u64, Error> {
        <T as PackedRead>::read_non_negative_binary_integer(self, None, None)
    }

    #[inline]
    fn read_bit_string(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error> {
        <T as BitRead>::read_bits_with_offset_len(self, buffer, bit_offset, bit_length)
    }

    #[inline]
    fn read_octet_string(&mut self, length_range: Option<(i64, i64)>) -> Result<Vec<u8>, Error> {
        <T as PackedRead>::read_octetstring(
            self,
            length_range.map(|v| v.0 as u64),
            length_range.map(|v| v.1 as u64),
            false,
        )
    }

    #[inline]
    fn read_bit_string_till_end(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
    ) -> Result<(), Error> {
        <T as BitRead>::read_bits_with_offset(self, buffer, bit_offset)
    }

    #[inline]
    fn read_length_determinant(&mut self) -> Result<usize, Error> {
        <T as PackedRead>::read_length_determinant(self, None, None).map(|v| v as usize)
    }

    #[inline]
    fn read_bit(&mut self) -> Result<bool, Error> {
        <T as BitRead>::read_bit(self)
    }
}

#[deprecated(note = "Use the UperReader/-Writer with the Read-/Writable interface instead")]
pub trait Writer {
    /// Sub-strings larger than 16k are not supported
    fn write_substring_with_length_determinant_prefix(
        &mut self,
        fun: &dyn Fn(&mut dyn Writer) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut buffer = BitBuffer::default();
        fun(&mut buffer as &mut dyn Writer)?;
        self.write_length_determinant(buffer.byte_len())?;
        self.write_bit_string(&buffer.content(), 0, buffer.bit_len())?;
        Ok(())
    }

    fn write_utf8_string(&mut self, value: &str) -> Result<(), Error>;
    /*{
        self.write_length_determinant(value.len())?;
        self.write_bit_string_till_end(value.as_bytes(), 0)?;
        Ok(())
    }*/

    fn write_choice_index_extensible(
        &mut self,
        index: u64,
        no_of_default_variants: u64,
    ) -> Result<(), Error>;
    /*{
        if index >= no_of_default_variants {
            self.write_bit(true)?;
            self.write_int_normally_small((index - no_of_default_variants) as u64)
        } else {
            self.write_bit(false)?;
            self.write_choice_index(index, no_of_default_variants)
        }
    }*/

    fn write_choice_index(&mut self, index: u64, no_of_default_variants: u64) -> Result<(), Error>;
    /*{
        self.write_int(index as i64, (0, no_of_default_variants as i64 - 1))
    }*/

    /// Range is inclusive
    fn write_int(&mut self, value: i64, range: (i64, i64)) -> Result<(), Error>;
    /*{
        let (lower, upper) = range;
        let value = {
            if value > upper || value < lower {
                return Err(Error::ValueNotInRange(value, lower, upper));
            }
            (value - lower) as u64
        };
        let leading_zeros = ((upper - lower) as u64).leading_zeros();

        let mut buffer = [0_u8; 8];
        NetworkEndian::write_u64(&mut buffer[..], value);
        let buffer_bits = buffer.len() * BYTE_LEN as usize;
        debug_assert!(buffer_bits == 64);

        self.write_bit_string_till_end(&buffer[..], leading_zeros as usize)?;

        Ok(())
    }*/

    fn write_int_normally_small(&mut self, value: u64) -> Result<(), Error>;
    /*{
        // X.691-201508 11.6
        if value <= 63 {
            // 11.6.1: '0'bit + 6 bit of the number
            self.write_bit(false)?;
            let buffer = value.to_be_bytes();
            self.write_bit_string(&buffer[7..8], 2, 6)?; // last 6 bits
            Ok(())
        } else if value <= i64::max_value() as u64 {
            // 11.6.2: '1'bit + (length-determinant + number)
            self.write_bit(true)?;
            self.write_int_max_unsigned(value as _)?;
            Ok(())
        } else {
            Err(Error::ValueExceedsMaxInt)
        }
    }*/

    /// ??? X.691-201508 11.9
    fn write_int_max_signed(&mut self, value: i64) -> Result<(), Error>;
    /*{
        let buffer = value.to_be_bytes();
        let mask = if value.is_negative() { 0xFF } else { 0x00 };
        let byte_len = {
            let mut len = buffer.len();
            while len > 0 && buffer[buffer.len() - len] == mask {
                len -= 1;
            }
            // otherwise one could not distinguish this positive value
            // from it being a totally different negative value
            if value.is_positive() && value.leading_zeros() % 8 == 0 {
                len += 1;
            }
            len
        }
        .max(1);
        self.write_length_determinant(byte_len)?;
        let bit_offset = (buffer.len() - byte_len) * BYTE_LEN;
        self.write_bit_string_till_end(&buffer, bit_offset)?;
        Ok(())
    }*/

    /// ??? X.691-201508 11.9
    fn write_int_max_unsigned(&mut self, value: u64) -> Result<(), Error>;
    /*{
        let buffer = value.to_be_bytes();
        let byte_len = {
            let mut len = buffer.len();
            while len > 0 && buffer[buffer.len() - len] == 0x00 {
                len -= 1;
            }
            len
        }
        .max(1);
        self.write_length_determinant(byte_len)?;
        let bit_offset = (buffer.len() - byte_len) * BYTE_LEN;
        self.write_bit_string_till_end(&buffer, bit_offset)?;
        Ok(())
    }*/

    fn write_bit_string(
        &mut self,
        buffer: &[u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error>;
    /* {
        if buffer.len() * BYTE_LEN < bit_offset || buffer.len() * BYTE_LEN < bit_offset + bit_length
        {
            return Err(Error::InsufficientDataInSourceBuffer);
        }
        for bit in bit_offset..bit_offset + bit_length {
            let byte_pos = bit / BYTE_LEN;
            let bit_pos = bit % BYTE_LEN;
            let bit_pos = BYTE_LEN - bit_pos - 1; // flip

            let bit = (buffer[byte_pos] >> bit_pos & 0x01) == 0x01;
            self.write_bit(bit)?;
        }
        Ok(())
    }*/

    fn write_octet_string(
        &mut self,
        string: &[u8],
        length_range: Option<(i64, i64)>,
    ) -> Result<(), Error>;
    /*{
        if let Some((min, max)) = length_range {
            self.write_int(string.len() as i64, (min, max))?;
        } else {
            self.write_length_determinant(string.len())?;
        }
        self.write_bit_string_till_end(string, 0)?;
        Ok(())
    }*/

    fn write_bit_string_till_end(&mut self, buffer: &[u8], bit_offset: usize) -> Result<(), Error>;
    /*{
        let len = (buffer.len() * BYTE_LEN) - bit_offset;
        self.write_bit_string(buffer, bit_offset, len)
    }*/

    fn write_length_determinant(&mut self, length: usize) -> Result<(), Error>;
    /*{
        if length <= UPER_LENGTH_DET_L1 as usize {
            self.write_bit(false)?;
            self.write_int(length as i64, (0, UPER_LENGTH_DET_L1))
        } else if length <= UPER_LENGTH_DET_L2 as usize {
            self.write_bit(true)?;
            self.write_bit(false)?;
            self.write_int(length as i64, (0, UPER_LENGTH_DET_L2))
        } else {
            Err(Error::UnsupportedOperation(format!(
                "Writing length determinant for lengths > {} is unsupported, tried for length {}",
                UPER_LENGTH_DET_L2, length
            )))
        }
    }*/

    fn write_bit(&mut self, bit: bool) -> Result<(), Error>;
}

#[allow(deprecated)]
impl<T: BitWrite<Error = Error> + PackedWrite<Error = Error>> Writer for T {
    #[inline]
    fn write_utf8_string(&mut self, value: &str) -> Result<(), Error> {
        <T as PackedWrite>::write_octetstring(self, None, None, false, value.as_bytes())
    }

    #[inline]
    fn write_choice_index_extensible(
        &mut self,
        index: u64,
        no_of_default_variants: u64,
    ) -> Result<(), Error> {
        <T as PackedWrite>::write_choice_index(self, no_of_default_variants, true, index)
    }

    #[inline]
    fn write_choice_index(&mut self, index: u64, no_of_default_variants: u64) -> Result<(), Error> {
        <T as PackedWrite>::write_choice_index(self, no_of_default_variants, false, index)
    }

    #[inline]
    fn write_int(
        &mut self,
        value: i64,
        (lower_bound, upper_bound): (i64, i64),
    ) -> Result<(), Error> {
        <T as PackedWrite>::write_constrained_whole_number(self, lower_bound, upper_bound, value)
    }

    #[inline]
    fn write_int_normally_small(&mut self, value: u64) -> Result<(), Error> {
        <T as PackedWrite>::write_normally_small_non_negative_whole_number(self, value)
    }

    #[inline]
    fn write_int_max_signed(&mut self, value: i64) -> Result<(), Error> {
        <T as PackedWrite>::write_unconstrained_whole_number(self, value)
    }

    #[inline]
    fn write_int_max_unsigned(&mut self, value: u64) -> Result<(), Error> {
        <T as PackedWrite>::write_non_negative_binary_integer(self, None, None, value)
    }

    #[inline]
    fn write_bit_string(
        &mut self,
        buffer: &[u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error> {
        <T as BitWrite>::write_bits_with_offset_len(self, buffer, bit_offset, bit_length)
    }

    #[inline]
    fn write_octet_string(
        &mut self,
        string: &[u8],
        length_range: Option<(i64, i64)>,
    ) -> Result<(), Error> {
        <T as PackedWrite>::write_octetstring(
            self,
            length_range.map(|v| v.0 as u64),
            length_range.map(|v| v.1 as u64),
            false,
            string,
        )
    }

    #[inline]
    fn write_bit_string_till_end(&mut self, buffer: &[u8], bit_offset: usize) -> Result<(), Error> {
        <T as BitWrite>::write_bits_with_offset(self, buffer, bit_offset)
    }

    #[inline]
    fn write_length_determinant(&mut self, length: usize) -> Result<(), Error> {
        <T as PackedWrite>::write_length_determinant(self, None, None, length as u64)
    }

    #[inline]
    fn write_bit(&mut self, bit: bool) -> Result<(), Error> {
        <T as BitWrite>::write_bit(self, bit)
    }
}
