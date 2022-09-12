use crate::syn::bitstring::BitVec;
use backtrace::Backtrace;
use byteorder::LittleEndian as E;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use std::io::Error as IoError;
use std::io::Read;
use std::io::Write;

mod peq;

pub use peq::ProtobufEq;

#[derive(Debug)]
pub enum Error {
    Io(Backtrace, IoError),
    #[allow(unused)]
    InvalidUtf8Received,
    #[allow(unused)]
    MissingRequiredField(&'static str),
    InvalidTagReceived(Backtrace, u32),
    InvalidFormat(Backtrace, u32),
    InvalidVariant(Backtrace, u64),
    UnexpectedFormat(Backtrace, Format),
    UnexpectedTag(Backtrace, (u32, Format)),
}

impl Error {
    #[allow(unused)]
    pub fn invalid_format(format: u32) -> Self {
        Error::InvalidFormat(Backtrace::new(), format)
    }

    #[allow(unused)]
    pub fn invalid_variant(variant: u64) -> Self {
        Error::InvalidVariant(Backtrace::new(), variant)
    }

    #[allow(unused)]
    pub fn invalid_tag_received(tag: u32) -> Self {
        Error::InvalidTagReceived(Backtrace::new(), tag)
    }

    #[allow(unused)]
    pub fn unexpected_format(format: Format) -> Self {
        Error::UnexpectedFormat(Backtrace::new(), format)
    }

    #[allow(unused)]
    pub fn unexpected_tag(tag: (u32, Format)) -> Self {
        Error::UnexpectedTag(Backtrace::new(), tag)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(b, ioe) => write!(f, "Internal IO Error: {}\n{:?}", ioe, b),
            Error::InvalidUtf8Received => write!(f, "Received String is not valid UTF8"),
            Error::MissingRequiredField(name) => {
                write!(f, "The required field '{}' is missing", name)
            }
            Error::InvalidTagReceived(b, tag) => write!(f, "Tag({}) is unknown\n{:?}", tag, b),
            Error::InvalidFormat(b, tag) => write!(f, "Format({}) is invalid\n{:?}", tag, b),
            Error::InvalidVariant(b, var) => write!(f, "Variant({}) is invalid\n{:?}", var, b),
            Error::UnexpectedFormat(b, format) => {
                write!(f, "Format({:?}) is unexpected\n{:?}", format, b)
            }
            Error::UnexpectedTag(b, (tag, format)) => {
                write!(f, "Tag({}/{:?}) is unexpected\n{:?}", tag, format, b)
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum Format {
    #[allow(unused)]
    VarInt = 0,
    #[allow(unused)]
    Fixed64 = 1,
    #[allow(unused)]
    LengthDelimited = 2,
    #[allow(unused)]
    Fixed32 = 5,
}

impl Format {
    #[allow(unused)]
    pub fn from(id: u32) -> Result<Format, Error> {
        match id {
            0 => Ok(Format::VarInt),
            1 => Ok(Format::Fixed64),
            2 => Ok(Format::LengthDelimited),
            5 => Ok(Format::Fixed32),
            f => Err(Error::InvalidFormat(Backtrace::new(), f)),
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(Backtrace::new(), e)
    }
}

pub trait ProtoWrite {
    fn write_varint(&mut self, value: u64) -> Result<(), Error>;

    fn write_bool(&mut self, value: bool) -> Result<(), Error> {
        self.write_varint(if value { 1 } else { 0 })
    }

    fn write_bytes(&mut self, value: &[u8]) -> Result<(), Error>;

    fn write_tag(&mut self, field: u32, format: Format) -> Result<(), Error> {
        self.write_varint(u64::from(field << 3 | (format as u32)))
    }

    fn write_enum_variant(&mut self, variant: u32) -> Result<(), Error> {
        self.write_varint(u64::from(variant))
    }

    fn write_sfixed32(&mut self, value: i32) -> Result<(), Error>;

    fn write_uint32(&mut self, value: u32) -> Result<(), Error> {
        self.write_varint(u64::from(value))
    }

    fn write_uint64(&mut self, value: u64) -> Result<(), Error> {
        self.write_varint(value)
    }

    fn write_sint32(&mut self, value: i32) -> Result<(), Error> {
        // remove leading negative sign to allow further size reduction
        // protobuf magic, probably something like value - I32_MIN
        self.write_varint(((value << 1) ^ (value >> 31)) as u64)
    }

    fn write_sint64(&mut self, value: i64) -> Result<(), Error> {
        // remove leading negative sign to allow further size reduction
        // protobuf magic, probably something like value - I64_MIN
        self.write_varint(((value << 1) ^ (value >> 63)) as u64)
    }

    fn write_string(&mut self, value: &str) -> Result<(), Error>;

    fn write_tagged_bool(&mut self, field: u32, value: bool) -> Result<(), Error> {
        self.write_tag(field, Format::VarInt)?;
        self.write_bool(value)
    }

    fn write_tagged_bytes(&mut self, field: u32, value: &[u8]) -> Result<(), Error> {
        self.write_tag(field, Format::LengthDelimited)?;
        self.write_bytes(value)
    }

    fn write_tagged_bit_vec(&mut self, field: u32, value: &BitVec) -> Result<(), Error> {
        let bytes = value.to_vec_with_trailing_bit_len();
        self.write_tagged_bytes(field, &bytes)
    }

    fn write_tagged_sfixed32(&mut self, field: u32, value: i32) -> Result<(), Error> {
        self.write_tag(field, Format::Fixed32)?;
        self.write_sfixed32(value)
    }

    fn write_tagged_uint32(&mut self, field: u32, value: u32) -> Result<(), Error> {
        self.write_tag(field, Format::VarInt)?;
        self.write_uint32(value)
    }

    fn write_tagged_uint64(&mut self, field: u32, value: u64) -> Result<(), Error> {
        self.write_tag(field, Format::VarInt)?;
        self.write_uint64(value)
    }

    fn write_tagged_sint32(&mut self, field: u32, value: i32) -> Result<(), Error> {
        self.write_tag(field, Format::VarInt)?;
        self.write_sint32(value)
    }

    fn write_tagged_sint64(&mut self, field: u32, value: i64) -> Result<(), Error> {
        self.write_tag(field, Format::VarInt)?;
        self.write_sint64(value)
    }

    fn write_tagged_string(&mut self, field: u32, value: &str) -> Result<(), Error> {
        self.write_tag(field, Format::LengthDelimited)?;
        self.write_string(value)
    }

    fn write_tagged_varint(&mut self, field: u32, value: u64) -> Result<(), Error> {
        self.write_tag(field, Format::VarInt)?;
        self.write_varint(value)
    }

    fn write_tagged_enum_variant(&mut self, field: u32, value: u32) -> Result<(), Error> {
        self.write_tagged_varint(field, u64::from(value))
    }
}

impl<W: Write> ProtoWrite for W {
    fn write_varint(&mut self, mut value: u64) -> Result<(), Error> {
        while value > 0x7F {
            self.write_u8(((value as u8) & 0x7F) | 0x80)?;
            value >>= 7;
        }
        self.write_u8(value as u8)?;
        Ok(())
    }

    fn write_bytes(&mut self, value: &[u8]) -> Result<(), Error> {
        self.write_varint(value.len() as u64)?;
        self.write_all(value)?;
        Ok(())
    }

    fn write_sfixed32(&mut self, value: i32) -> Result<(), Error> {
        self.write_i32::<E>(value)?;
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> Result<(), Error> {
        self.write_bytes(value.as_bytes())?;
        Ok(())
    }
}

pub trait ProtoRead {
    fn read_varint(&mut self) -> Result<u64, Error>;

    fn read_bool(&mut self) -> Result<bool, Error> {
        Ok(self.read_varint()? != 0)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, Error>;

    fn read_bit_vec(&mut self) -> Result<BitVec, Error> {
        let bytes = self.read_bytes()?;
        Ok(BitVec::from_vec_with_trailing_bit_len(bytes))
    }

    fn read_tag(&mut self) -> Result<(u32, Format), Error> {
        let mask = 0b0000_0111;
        let tag = self.read_varint()? as u32;
        let format = Format::from(tag & mask)?;
        let field = tag >> 3;
        Ok((field, format))
    }

    fn read_enum_variant(&mut self) -> Result<u32, Error> {
        Ok(self.read_varint()? as u32)
    }

    fn read_sfixed32(&mut self) -> Result<i32, Error>;

    fn read_uint32(&mut self) -> Result<u32, Error> {
        Ok(self.read_varint()? as u32)
    }

    fn read_uint64(&mut self) -> Result<u64, Error> {
        self.read_varint()
    }

    fn read_sint32(&mut self) -> Result<i32, Error> {
        // remove leading negative sign to allow further size reduction
        // protobuf magic, probably something like value - I32_MIN
        let value = self.read_varint()? as u32;
        Ok(((value >> 1) as i32) ^ (-((value & 0x01) as i32)))
    }

    fn read_sint64(&mut self) -> Result<i64, Error> {
        // remove leading negative sign to allow further size reduction
        // protobuf magic, probably something like value - I64_MIN
        let value = self.read_varint()?;
        Ok(((value >> 1) as i64) ^ (-((value & 0x01) as i64)))
    }

    fn read_string(&mut self) -> Result<String, Error>;
}

impl<R: Read> ProtoRead for R {
    fn read_varint(&mut self) -> Result<u64, Error> {
        let mut value = 0;
        let mut shift = 0_usize;
        while shift < 64 {
            let read = self.read_u8()?;
            value |= u64::from(read & 0x7F) << shift;
            shift += 7;
            if read & 0x80 == 0 {
                break;
            }
        }
        Ok(value)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, Error> {
        let mut vec = Vec::new();
        self.read_to_end(&mut vec)?;
        Ok(vec)
    }

    fn read_sfixed32(&mut self) -> Result<i32, Error> {
        Ok(self.read_i32::<E>()?)
    }

    fn read_string(&mut self) -> Result<String, Error> {
        let bytes = self.read_bytes()?;
        if let Ok(string) = String::from_utf8(bytes) {
            Ok(string)
        } else {
            Err(Error::InvalidUtf8Received)
        }
    }
}
