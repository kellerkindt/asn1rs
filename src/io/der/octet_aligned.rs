use crate::io::buf::OctetBuffer;
use crate::io::der::Error;
use crate::io::der::{DistinguishedRead, DistinguishedWrite};
use crate::model::Tag;

#[derive(Debug)]
pub enum PC {
    Primitive = 0,
    Constructed,
}
impl From<bool> for PC {
    fn from(v: bool) -> Self {
        match v {
            false => Self::Primitive,
            true => Self::Constructed,
        }
    }
}

#[derive(Debug)]
pub enum Length {
    Indefinite,
    Definite(usize),
    Reserved,
}

impl DistinguishedRead for OctetBuffer {
    fn read_octet(&mut self) -> Result<u8, Error> {
        if self.read_position > self.buffer.len() {
            return Err(Error::EndOfStream);
        }
        let octet = self.buffer[self.read_position];
        self.read_position += 1;
        Ok(octet)
    }

    fn read_octets_with_len(&mut self, dst: &mut [u8], dst_len: usize) -> Result<(), Error> {
        if self.read_position + dst_len > self.buffer.len() {
            return Err(Error::EndOfStream);
        }
        dst.copy_from_slice(
            &self.buffer[self.read_position..(self.read_position + dst_len) as usize],
        );
        // dst[..dst_len] = self.buffer[self.read_position..self.read_position + dst_len];
        self.read_position += dst_len;
        Ok(())
    }

    fn read_octets(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        let dst_len = dst.len();
        self.read_octets_with_len(dst, dst_len)
    }

    fn read_identifier(&mut self) -> Result<(Tag, PC), Error> {
        let octet = self.read_octet()?;
        let class_bits = (octet >> 6) & 0x3;

        let pc_bit = (octet >> 5) & 0x1 == 1;
        let pc = PC::from(pc_bit);

        let tag_number = (octet & 0x1F) as usize;

        // TODO: Support for log tags

        // TODO: Parse tag as type : https://en.wikipedia.org/wiki/X.690#Types

        let tag = match class_bits {
            0 => Tag::Universal(tag_number),
            1 => Tag::Application(tag_number),
            2 => Tag::ContextSpecific(tag_number),
            3 => Tag::Private(tag_number),
            _ => unreachable!(),
        };

        Ok((tag, pc))
    }

    fn read_length(&mut self) -> Result<Length, Error> {
        let octet = self.read_octet()?;
        let msb = (octet >> 7) & 0x1 == 1;

        let length_number = (octet & 0x7f) as usize;

        if !msb {
            return Ok(Length::Definite(length_number));
        }

        match length_number {
            0 => Ok(Length::Indefinite),
            127 => Ok(Length::Reserved),
            _ => {
                let mut length_bits = [0u8; std::mem::size_of::<usize>()];
                let offset = length_bits.len() - length_number;
                self.read_octets(&mut length_bits[offset..])?;
                Ok(Length::Definite(usize::from_be_bytes(length_bits)))
            }
        }
    }

    fn read_i64_number(&mut self, length: usize) -> Result<i64, Error> {
        let mut bytes = [0u8; std::mem::size_of::<i64>()];
        let offset = bytes.len() - length as usize;
        self.read_octets(&mut bytes[offset..])?;
        Ok(i64::from_be_bytes(bytes))
    }

    fn read_octet_string(&mut self, length: usize) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![0u8; length];
        self.read_octets(&mut buffer[..])?;
        Ok(buffer)
    }
}

impl DistinguishedWrite for OctetBuffer {}
