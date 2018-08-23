pub const BYTE_LEN: usize = 8;

#[derive(Debug)]
pub enum Error {
    InvalidUtf8String,
    UnsupportedOperation(String),
    InsufficientSpaceInDestinationBuffer,
    InsufficientDataInSourceBuffer,
    ValueNotInRange(i64, i64, i64),
    EndOfStream,
}

pub trait Uper {
    fn read_uper(reader: &mut Reader) -> Result<Self, Error>
    where
        Self: Sized;

    fn write_uper(&self, writer: &mut Writer) -> Result<(), Error>;
}

pub trait Reader {
    fn read_utf8_string(&mut self) -> Result<String, Error>;

    fn read_int(&mut self, range: (i64, i64)) -> Result<i64, Error>;

    fn read_int_max(&mut self) -> Result<u64, Error>;

    fn read_bit_string(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error>;

    fn read_bit_string_till_end(
        &mut self,
        buffer: &mut [u8],
        bit_offset: usize,
    ) -> Result<(), Error> {
        let len = (buffer.len() * BYTE_LEN) - bit_offset;
        self.read_bit_string(buffer, bit_offset, len)
    }

    fn read_length_determinant(&mut self) -> Result<usize, Error>;

    fn read_bit(&mut self) -> Result<bool, Error>;
}

pub trait Writer {
    fn write_utf8_string(&mut self, value: &str) -> Result<(), Error>;

    fn write_int(&mut self, value: i64, range: (i64, i64)) -> Result<(), Error>;

    fn write_int_max(&mut self, value: u64) -> Result<(), Error>;

    fn write_bit_string(
        &mut self,
        buffer: &[u8],
        bit_offset: usize,
        bit_length: usize,
    ) -> Result<(), Error>;

    fn write_bit_string_till_end(&mut self, buffer: &[u8], bit_offset: usize) -> Result<(), Error> {
        let len = (buffer.len() * BYTE_LEN) - bit_offset;
        self.write_bit_string(buffer, bit_offset, len)
    }

    fn write_length_determinant(&mut self, length: usize) -> Result<(), Error>;

    fn write_bit(&mut self, bit: bool) -> Result<(), Error>;
}
