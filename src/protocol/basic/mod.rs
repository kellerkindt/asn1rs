//! This module contains defines traits to encode and decode basic ASN.1 primitives and types of
//! the basic family (BER, DER, CER).

mod distinguished;
mod err;

pub use distinguished::*;
pub use err::Error;

use asn1rs_model::asn::Tag;

/// According to ITU-T X.690
pub trait BasicRead {
    type Flavor;

    /// According to ITU-T X.690, chapter 8.1.2, an identifier octet contains the class and number
    /// of the type.
    fn read_identifier(&mut self) -> Result<Tag, Error>;

    /// According to ITU-T X.690, chapter 8.1.3, the length is encoded in at least one byte, in
    /// either the short (8.1.3.4) or long (8.1.3.5) form
    fn read_length(&mut self) -> Result<usize, Error>;

    /// According to ITU-T X.690, chapter 8.2, the boolean type is represented in a single byte,
    /// where 0 represents `false` and any other value represents `true`.
    fn read_boolean(&mut self) -> Result<bool, Error>;
}

/// According to ITU-T X.690
pub trait BasicWrite {
    type Flavor;

    /// According to ITU-T X.690, chapter 8.1.2, an identifier octet contains the class and number
    /// of the type.
    fn write_identifier(&mut self, tag: Tag) -> Result<(), Error>;

    /// According to ITU-T X.690, chapter 8.1.3, the length is encoded in at least one byte, in
    /// either the short (8.1.3.4) or long (8.1.3.5) form
    fn write_length(&mut self, length: usize) -> Result<(), Error>;

    /// According to ITU-T X.690, chapter 8.2, the boolean type is represented in a single byte,
    /// where 0 represents `false` and any other value represents `true`.
    fn write_boolean(&mut self, value: bool) -> Result<(), Error>;
}
