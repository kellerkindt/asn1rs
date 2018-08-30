use backtrace::Backtrace;
use byteorder::LittleEndian as E;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

use std::io::Error as IoError;
use std::io::Read;
use std::io::Write;

#[derive(Debug)]
pub enum Error {
    Io(Backtrace, IoError),
    #[allow(unused)]
    InvalidUtf8Received,
    #[allow(unused)]
    MissingRequiredField(&'static str),
    InvalidTagReceived(Backtrace, u32),
    InvalidFormat(Backtrace, u32),
    UnexpectedFormat(Backtrace, Format),
    UnexpectedTag(Backtrace, (u32, Format)),
}

impl Error {
    #[allow(unused)]
    pub fn invalid_format(format: u32) -> Self {
        Error::InvalidFormat(Backtrace::new(), format)
    }

    #[allow(unused)]
    pub fn invalid_variant(format: u32) -> Self {
        Error::InvalidFormat(Backtrace::new(), format)
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

#[derive(Debug, PartialOrd, PartialEq)]
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

impl ToString for Format {
    fn to_string(&self) -> String {
        match self {
            Format::VarInt => "VarInt",
            Format::Fixed64 => "Fixed64",
            Format::LengthDelimited => "LengthDelimited",
            Format::Fixed32 => "Fixed32",
        }.into()
    }
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

pub trait Protobuf: ProtobufEq {
    fn protobuf_format(&self) -> Format;

    fn read_protobuf(reader: &mut Reader) -> Result<Self, Error>
    where
        Self: Sized;

    fn write_protobuf(&self, writer: &mut Writer) -> Result<(), Error>;
}

pub trait Writer {
    fn write_varint(&mut self, value: u64) -> Result<(), Error>;

    fn write_bool(&mut self, value: bool) -> Result<(), Error> {
        self.write_varint(if value { 1 } else { 0 })
    }

    fn write_bytes(&mut self, value: &[u8]) -> Result<(), Error>;

    fn write_tag(&mut self, field: u32, format: Format) -> Result<(), Error> {
        self.write_varint((field << 3 | (format as u32)) as u64)
    }

    fn write_enum_variant(&mut self, variant: u32) -> Result<(), Error> {
        self.write_varint(variant as u64)
    }

    fn write_sfixed32(&mut self, value: i32) -> Result<(), Error>;

    fn write_uint32(&mut self, value: u32) -> Result<(), Error> {
        self.write_varint(value as u64)
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
        self.write_tag(field, Format::VarInt)?;
        self.write_bytes(value)
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
        self.write_tagged_varint(field, value as u64)
    }
}

impl<W: Write> Writer for W {
    fn write_varint(&mut self, mut value: u64) -> Result<(), Error> {
        while value > 0x7F {
            self.write_u8(((value as u8) & 0x7F) | 0x80)?;
            value >>= 7;
        }
        Ok(self.write_u8(value as u8)?)
    }

    fn write_bytes(&mut self, value: &[u8]) -> Result<(), Error> {
        self.write_varint(value.len() as u64)?;
        self.write_all(&value)?;
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

pub trait Reader {
    fn read_varint(&mut self) -> Result<u64, Error>;

    fn read_bool(&mut self) -> Result<bool, Error> {
        Ok(self.read_varint()? != 0)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, Error>;

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

impl<R: Read> Reader for R {
    fn read_varint(&mut self) -> Result<u64, Error> {
        let mut value = 0;
        let mut shift = 0_usize;
        while shift < 64 {
            let read = self.read_u8()?;
            value |= ((read & 0x7F) as u64) << shift;
            shift += 7;
            if read & 0x80 == 0 {
                break;
            }
        }
        Ok(value)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.read_varint()? as usize;
        let mut vec = vec![0u8; len];
        self.read_exact(&mut vec[..])?;
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

pub trait ProtobufEq<Rhs: ?Sized = Self> {
    fn protobuf_eq(&self, other: &Rhs) -> bool;
}

impl<T: ProtobufEq + Default + PartialEq> ProtobufEq<Option<T>> for Option<T> {
    fn protobuf_eq(&self, other: &Option<T>) -> bool {
        match self {
            Some(ref v) => match other {
                Some(ref v_other) => v.protobuf_eq(v_other),
                None => v == &T::default(),
            },
            None => match other {
                Some(ref v_other) => &T::default() == v_other,
                None => true,
            },
        }
    }
}

impl<T: ProtobufEq> ProtobufEq<Vec<T>> for Vec<T> {
    fn protobuf_eq(&self, other: &Vec<T>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            for (i, v) in self.iter().enumerate() {
                if !other[i].protobuf_eq(v) {
                    return false;
                }
            }
            true
        }
    }
}

impl ProtobufEq<bool> for bool {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<u8> for u8 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<u16> for u16 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<u32> for u32 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<u64> for u64 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<i8> for i8 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<i16> for i16 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<i32> for i32 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<i64> for i64 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<String> for String {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}
