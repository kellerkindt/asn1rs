pub mod octet_aligned;

// in the long term, this Error type should be moved - maybe to crate::io::err::Error ?
pub use super::per::Error;
use crate::io::der::octet_aligned::{Length, PC};
use crate::model::Tag;

/// According to ITU-TX.690 | ISO/IEC 8825-1:2015
pub trait DistinguishedRead {
    fn read_octet(&mut self) -> Result<u8, Error>;
    fn read_octets_with_len(&mut self, dst: &mut [u8], dst_len: usize) -> Result<(), Error>;
    fn read_octets(&mut self, dst: &mut [u8]) -> Result<(), Error>;
    fn read_identifier(&mut self, expected_tag: Tag) -> Result<(Tag, PC), Error>;
    fn read_length(&mut self) -> Result<Length, Error>;
    fn read_i64_number(&mut self, length: usize) -> Result<i64, Error>;
    fn read_octet_string(&mut self, length: usize) -> Result<Vec<u8>, Error>;
}

/// According to ITU-TX.690 | ISO/IEC 8825-1:2015
pub trait DistinguishedWrite {}
