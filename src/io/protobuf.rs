use byteorder::LittleEndian as E;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

use std::io::Error as IoError;
use std::io::Read;
use std::io::Write;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    #[allow(unused)]
    InvalidUtf8Received,
    #[allow(unused)]
    MissingRequiredField(&'static str),
    #[allow(unused)]
    InvalidTagReceived(u32),
    InvalidFormat(u32),
}

#[derive(Debug)]
#[repr(u32)]
pub enum Format {
    VARINT = 0,
    FIXED64 = 1,
    LENGTH_DELIMITED = 2,
    FIXED32 = 5,
}

impl Format {
    pub fn from(id: u32) -> Result<Format, Error> {
        match id {
            0 => Ok(Format::VARINT),
            1 => Ok(Format::FIXED64),
            5 => Ok(Format::FIXED32),
            f => Err(Error::InvalidFormat(f)),
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(e)
    }
}

pub trait Protobuf {
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

    fn write_uint64(&mut self, value: u64) -> Result<(), Error>;

    fn write_string(&mut self, value: &str) -> Result<(), Error>;

    fn write_tagged_bool(&mut self, field: u32, value: bool) -> Result<(), Error> {
        self.write_tag(field, Format::VARINT)?;
        self.write_bool(value)
    }

    fn write_tagged_sfixed32(&mut self, field: u32, value: i32) -> Result<(), Error> {
        self.write_tag(field, Format::FIXED32)?;
        self.write_sfixed32(value)
    }

    fn write_tagged_uint64(&mut self, field: u32, value: u64) -> Result<(), Error> {
        self.write_tag(field, Format::FIXED64)?;
        self.write_uint64(value)
    }

    fn write_tagged_string(&mut self, field: u32, value: &str) -> Result<(), Error> {
        self.write_tag(field, Format::LENGTH_DELIMITED)?;
        self.write_string(value)
    }

    fn write_tagged_varint(&mut self, field: u32, value: u64) -> Result<(), Error> {
        self.write_tag(field, Format::VARINT)?;
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

    fn write_uint64(&mut self, value: u64) -> Result<(), Error> {
        self.write_u64::<E>(value)?;
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

    fn read_uint64(&mut self) -> Result<u64, Error>;

    fn read_string(&mut self) -> Result<String, Error>;
}

impl<R: Read> Reader for R {
    fn read_varint(&mut self) -> Result<u64, Error> {
        let mut value = 0;
        loop {
            let read = self.read_u8()?;
            value <<= 7;
            value |= (read & 0x7F) as u64;
            if read & 0x80 == 0 {
                break;
            }
        }
        Ok(value)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.read_varint()? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(self.read_u8()?);
        }
        Ok(vec)
    }

    fn read_sfixed32(&mut self) -> Result<i32, Error> {
        Ok(self.read_i32::<E>()?)
    }

    fn read_uint64(&mut self) -> Result<u64, Error> {
        Ok(self.read_u64::<E>()?)
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
