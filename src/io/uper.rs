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
    ValueNotInRange(i64, i64, i64),
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
            Error::ValueNotInRange(value, min, max) => write!(
                f,
                "The value {} is not within the inclusive range of {} and {}",
                value, min, max
            ),
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

pub trait Uper {
    fn read_uper(reader: &mut dyn Reader) -> Result<Self, Error>
    where
        Self: Sized;

    fn write_uper(&self, writer: &mut dyn Writer) -> Result<(), Error>;
}

pub trait Reader {
    fn read_utf8_string(&mut self) -> Result<String, Error> {
        let len = self.read_length_determinant()?;
        let mut buffer = vec![0_u8; len];
        self.read_bit_string_till_end(&mut buffer[..len], 0)?;
        if let Ok(string) = String::from_utf8(buffer) {
            Ok(string)
        } else {
            Err(Error::InvalidUtf8String)
        }
    }

    fn read_int(&mut self, range: (i64, i64)) -> Result<i64, Error> {
        let (lower, upper) = range;
        let leading_zeros = ((upper - lower) as u64).leading_zeros();

        let mut buffer = [0_u8; 8];
        let buffer_bits = buffer.len() * BYTE_LEN as usize;
        debug_assert!(buffer_bits == 64);
        self.read_bit_string_till_end(&mut buffer[..], leading_zeros as usize)?;
        let value = NetworkEndian::read_u64(&buffer[..]) as i64;
        Ok(value + lower)
    }

    fn read_int_max(&mut self) -> Result<u64, Error> {
        let len_in_bytes = self.read_length_determinant()?;
        if len_in_bytes > 8 {
            Err(Error::UnsupportedOperation(
                "Reading bigger data types than 64bit is not supported".into(),
            ))
        } else {
            let mut buffer = vec![0_u8; 8];
            let offset = (8 * BYTE_LEN) - (len_in_bytes * BYTE_LEN);
            self.read_bit_string_till_end(&mut buffer[..], offset)?;
            Ok(NetworkEndian::read_u64(&buffer[..]))
        }
    }

    fn read_bit_string(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error> {
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
    }

    fn read_octet_string(&mut self, length_range: Option<(i64, i64)>) -> Result<Vec<u8>, Error> {
        let len = if let Some((min, max)) = length_range {
            self.read_int((min, max))? as usize
        } else {
            self.read_length_determinant()?
        };
        let mut vec = vec![0_u8; len];
        self.read_bit_string_till_end(&mut vec[..], 0)?;
        Ok(vec)
    }

    fn read_bit_string_till_end(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
    ) -> Result<(), Error> {
        let len = (buffer.len() * BYTE_LEN) - bit_offset;
        self.read_bit_string(buffer, bit_offset, len)
    }

    #[allow(clippy::if_not_else)]
    fn read_length_determinant(&mut self) -> Result<usize, Error> {
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
    }

    fn read_bit(&mut self) -> Result<bool, Error>;
}

pub trait Writer {
    fn write_utf8_string(&mut self, value: &str) -> Result<(), Error> {
        self.write_length_determinant(value.len())?;
        self.write_bit_string_till_end(value.as_bytes(), 0)?;
        Ok(())
    }

    fn write_int(&mut self, value: i64, range: (i64, i64)) -> Result<(), Error> {
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
    }

    fn write_int_max(&mut self, value: u64) -> Result<(), Error> {
        if value > i64::max_value() as u64 {
            return Err(Error::ValueNotInRange(value as i64, 0, i64::max_value()));
        }
        let mut buffer = [0_u8; 8];
        NetworkEndian::write_u64(&mut buffer[..], value);
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
    }

    fn write_bit_string(
        &mut self,
        buffer: &[u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error> {
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
    }

    fn write_octet_string(
        &mut self,
        string: &[u8],
        length_range: Option<(i64, i64)>,
    ) -> Result<(), Error> {
        if let Some((min, max)) = length_range {
            self.write_int(string.len() as i64, (min, max))?;
        } else {
            self.write_length_determinant(string.len())?;
        }
        self.write_bit_string_till_end(string, 0)?;
        Ok(())
    }

    fn write_bit_string_till_end(&mut self, buffer: &[u8], bit_offset: usize) -> Result<(), Error> {
        let len = (buffer.len() * BYTE_LEN) - bit_offset;
        self.write_bit_string(buffer, bit_offset, len)
    }

    fn write_length_determinant(&mut self, length: usize) -> Result<(), Error> {
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
    }

    fn write_bit(&mut self, bit: bool) -> Result<(), Error>;
}
