use crate::io::buf::OctetBuffer;
use crate::io::der::Error;
use crate::io::der::{DistinguishedRead, DistinguishedWrite};
use crate::io::per::unaligned::BitRead;

impl BitRead for OctetBuffer {
    #[inline]
    fn read_bit(&mut self) -> Result<bool, Error> {
        if self.read_position < self.write_position {
            BitRead::read_bit(&mut (&self.buffer[..], &mut self.read_position))
        } else {
            Err(Error::EndOfStream)
        }
    }

    #[inline]
    fn read_bits(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        BitRead::read_bits(&mut (&self.buffer[..], &mut self.read_position), dst)
    }

    #[inline]
    fn read_bits_with_offset(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
    ) -> Result<(), Error> {
        BitRead::read_bits_with_offset(
            &mut (&self.buffer[..], &mut self.read_position),
            dst,
            dst_bit_offset,
        )
    }

    #[inline]
    fn read_bits_with_len(&mut self, dst: &mut [u8], dst_bit_len: usize) -> Result<(), Error> {
        BitRead::read_bits_with_len(
            &mut (&self.buffer[..], &mut self.read_position),
            dst,
            dst_bit_len,
        )
    }

    #[inline]
    fn read_bits_with_offset_len(
        &mut self,
        dst: &mut [u8],
        dst_bit_offset: usize,
        dst_bit_len: usize,
    ) -> Result<(), Error> {
        BitRead::read_bits_with_offset_len(
            &mut (&self.buffer[..], &mut self.read_position),
            dst,
            dst_bit_offset,
            dst_bit_len,
        )
    }
}

#[derive(Debug)]
pub enum Class {
    Universal = 0,
    Application,
    ContextSpecific,
    Private
}
impl From<u8> for Class {
    fn from(v: u8) -> Self {
        match v {
            x if x == Self::Universal as u8 => Self::Universal,
            x if x == Self::Application as u8 => Self::Application,
            x if x == Self::ContextSpecific as u8 => Self::ContextSpecific,
            x if x == Self::Private as u8 => Self::Private,
            _ => Self::Universal
        }
    }
}

#[derive(Debug)]
pub enum PC {
    Primitive = 0,
    Constructed
}
impl From<bool> for PC {
    fn from(v: bool) -> Self {
        match v {
            x if x == false => Self::Primitive,
            x if x == true => Self::Constructed,
            _ => Self::Constructed
        }
    }
}

#[derive(Debug)]
pub enum Length {
    Indefinite,
    Definite(usize),
    Reserved
}

impl DistinguishedRead for OctetBuffer {

    fn read_identifier(&mut self) -> Result<(Class, PC, u8), Error> {

        let bit8 = self.read_bit()?;
        let bit7 = self.read_bit()?;
        let class = Class::from(((bit8 as u8) << 1) + (bit7 as u8));

        let bit6 = self.read_bit()?;
        let pc = PC::from(bit6);

        let mut tag_bits = vec![0u8; 1];
        self.read_bits_with_len(&mut tag_bits[..], 5)?;
        let tag_number = (tag_bits[0] as u8) >> 3;

        // TODO: Support for log tags

        // TODO: Parse tag as type

        Ok((class, pc, tag_number))
    }

    fn read_length(&mut self) -> Result<Length, Error> {

        let bit8 = self.read_bit()?;

        let mut length_bits = vec![0u8; 1];
        self.read_bits_with_len(&mut length_bits[..], 7)?;
        let length_number = (length_bits[0] as usize) >> 1;

        if !bit8 {
            return Ok(Length::Definite(length_number))
        }

        match length_number {
            0 => Ok(Length::Indefinite),
            127 => Ok(Length::Reserved),
            _ => {
                let mut length_bits = [0u8; std::mem::size_of::<usize>()];
                let start = &length_bits.len()-length_number;
                self.read_bits_with_len(&mut length_bits[start..], length_number*8)?;
                Ok(Length::Definite(usize::from_be_bytes(length_bits)))
            }
        }
    }

    fn read_i64_number(&mut self, length: usize) -> Result<i64, Error> {
        let mut bytes = [0u8; std::mem::size_of::<i64>()];
        let offset = bytes.len() - length as usize;
        self.read_bits(&mut bytes[offset..])?;
        Ok(i64::from_be_bytes(bytes))
    }

    fn read_octet_string(&mut self, length: usize) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![0u8; length];
        self.read_bits(&mut buffer[..])?;
        Ok(buffer)
    }

}

impl DistinguishedWrite for OctetBuffer {}
