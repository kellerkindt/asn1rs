pub mod buffer;
pub mod protobuf;
pub mod uper;

use std::fmt::Debug;

pub trait Codec {
    type Error: Debug;
    type Reader: CodecReader + ?Sized;
    type Writer: CodecWriter + ?Sized;
}

pub trait CodecReader {}
impl<R: ::std::io::Read> CodecReader for R {}

pub trait CodecWriter {}
impl<W: ::std::io::Write> CodecWriter for W {}

pub trait Serializable<C: Codec> {
    fn write(&self, writer: &mut C::Writer) -> Result<(), C::Error>;

    fn read(reader: &mut C::Reader) -> Result<Self, C::Error>
    where
        Self: Sized;
}
