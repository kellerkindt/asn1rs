use crate::io::buf::OctetBuffer;
use crate::io::der::DistinguishedRead;
use crate::io::der::DistinguishedWrite;
use crate::io::der::Error;
use crate::prelude::*;
use crate::io::der::octet_aligned::{Length, Class};
use crate::io::per::unaligned::BitRead;

#[derive(Default)]
pub struct DerWriter {
    buffer: OctetBuffer,
}

impl DerWriter {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: OctetBuffer::with_capacity(capacity),
        }
    }

    pub fn byte_content(&self) -> &[u8] {
        self.buffer.content()
    }

    pub fn into_bytes_vec(self) -> Vec<u8> {
        self.buffer.into()
    }

    pub fn into_reader(self) -> DerReader {
        DerReader::from_bits(self.into_bytes_vec())
    }
}

impl Writer for DerWriter {
    type Error = Error;

    #[inline]
    fn write_sequence<C: sequence::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_sequence_of<C: sequenceof::Constraint, T: WritableType>(
        &mut self,
        slice: &[T::Type],
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn write_set<C: set::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(&mut self, f: F) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn write_set_of<C: setof::Constraint, T: WritableType>(&mut self, slice: &[<T as WritableType>::Type]) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_enumerated<C: enumerated::Constraint>(
        &mut self,
        enumerated: &C,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_choice<C: choice::Constraint>(&mut self, choice: &C) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_opt<T: WritableType>(
        &mut self,
        value: Option<&<T as WritableType>::Type>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_number<T: numbers::Number, C: numbers::Constraint<T>>(
        &mut self,
        value: T,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_utf8string<C: utf8string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_ia5string<C: ia5string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_octet_string<C: octetstring::Constraint>(
        &mut self,
        value: &[u8],
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_bit_string<C: bitstring::Constraint>(
        &mut self,
        value: &[u8],
        bit_len: u64,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn write_boolean<C: boolean::Constraint>(&mut self, value: bool) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

pub struct DerReader {
    buffer: OctetBuffer,
}

impl DerReader {
    pub fn from_bits<I: Into<Vec<u8>>>(bytes: I) -> Self {
        Self {
            buffer: OctetBuffer::from_bytes(bytes.into()),
        }
    }

    #[inline]
    pub const fn bytes_remaining(&self) -> usize {
        self.buffer.write_position - self.buffer.read_position
    }
}

impl Reader for DerReader {
    type Error = Error;

    #[inline]
    fn read_sequence<
        C: sequence::Constraint,
        S: Sized,
        F: Fn(&mut Self) -> Result<S, Self::Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<S, Self::Error> {
        let (class, pc, tag) = self.buffer.read_identifier()?;
        let length = self.buffer.read_length()?;

        eprintln!("Class = {:#?}, PC = {:#?}, Tag = {:#?}, Length = {:#?}", class, pc, tag, length);

        // TODO: Why?!
        if let Class::Application = class {
            return self.read_sequence::<C, S, F>(f);
        }

        let range = match length {
            Length::Definite(l) => Some(self.buffer.read_position..self.buffer.read_position + l as usize),
            Length::Indefinite => Some(self.buffer.read_position..self.buffer.write_position),
            _ => None
        };

        if let Some(ref range1) = range {
            if self.buffer.byte_len()*8 < range1.end {
                return Err(Error::EndOfStream);
            }
        }

        f(self)
    }

    #[inline]
    fn read_sequence_of<C: sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<T::Type>, Self::Error> {
        unimplemented!()
    }

    fn read_set<C: set::Constraint, S: Sized, F: Fn(&mut Self) -> Result<S, Self::Error>>(&mut self, f: F) -> Result<S, Self::Error> {
        self.read_sequence::<C, S, F>(f)
    }

    fn read_set_of<C: setof::Constraint, T: ReadableType>(&mut self) -> Result<Vec<<T as ReadableType>::Type>, Self::Error> {
        self.read_sequence_of::<C, T>()
    }

    #[inline]
    fn read_enumerated<C: enumerated::Constraint>(&mut self) -> Result<C, Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn read_choice<C: choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn read_opt<T: ReadableType>(
        &mut self,
    ) -> Result<Option<<T as ReadableType>::Type>, Self::Error> {
        Ok(None)
    }

    #[inline]
    fn read_number<T: numbers::Number, C: numbers::Constraint<T>>(
        &mut self,
    ) -> Result<T, Self::Error> {
        let (class, pc, tag) = self.buffer.read_identifier()?;
        let length = self.buffer.read_length()?;

        eprintln!("Class = {:#?}, PC = {:#?}, Tag = {:#?}, Length = {:#?}", class, pc, tag, length);

        if let Length::Definite(l) = length {
            self.buffer.read_i64_number(l).map(T::from_i64)
        } else {
            Err(Error::UnsupportedOperation("Indefinite range is not supported in DER".to_string()))
        }
    }

    #[inline]
    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let (class, pc, tag) = self.buffer.read_identifier()?;
        let length = self.buffer.read_length()?;

        eprintln!("Class = {:#?}, PC = {:#?}, Tag = {:#?}, Length = {:#?}", class, pc, tag, length);

        if let Length::Definite(l) = length {
            let octets = self.buffer.read_octet_string(l)?;
            String::from_utf8(octets).map_err(|_| Self::Error::InvalidUtf8String)
        } else {
            Err(Error::UnsupportedOperation("Indefinite range is not supported in DER".to_string()))
        }
    }

    #[inline]
    fn read_ia5string<C: ia5string::Constraint>(&mut self) -> Result<String, Self::Error> {
        let (class, pc, tag) = self.buffer.read_identifier()?;
        let length = self.buffer.read_length()?;

        eprintln!("Class = {:#?}, PC = {:#?}, Tag = {:#?}, Length = {:#?}", class, pc, tag, length);

        if let Length::Definite(l) = length {
            let octets = self.buffer.read_octet_string(l)?;
            String::from_utf8(octets).map_err(|_| Self::Error::InvalidUtf8String)
        } else {
            Err(Error::UnsupportedOperation("Indefinite range is not supported in DER".to_string()))
        }
    }

    #[inline]
    fn read_octet_string<C: octetstring::Constraint>(&mut self) -> Result<Vec<u8>, Self::Error> {
        let (class, pc, tag) = self.buffer.read_identifier()?;
        let length = self.buffer.read_length()?;

        eprintln!("Class = {:#?}, PC = {:#?}, Tag = {:#?}, Length = {:#?}", class, pc, tag, length);

        if let Length::Definite(l) = length {
            self.buffer.read_octet_string(l)
        } else {
            Err(Error::UnsupportedOperation("Indefinite range is not supported in DER".to_string()))
        }
    }

    #[inline]
    fn read_bit_string<C: bitstring::Constraint>(&mut self) -> Result<(Vec<u8>, u64), Self::Error> {
        let (class, pc, tag) = self.buffer.read_identifier()?;
        let length = self.buffer.read_length()?;

        eprintln!("Class = {:#?}, PC = {:#?}, Tag = {:#?}, Length = {:#?}", class, pc, tag, length);

        if let Length::Definite(l) = length {
            let octets = self.buffer.read_octet_string(l)?;
            let size = (&octets.len() * 8) as u64;
            Ok((octets, size))
        } else {
            Err(Error::UnsupportedOperation("Indefinite range is not supported in DER".to_string()))
        }
    }

    #[inline]
    fn read_boolean<C: boolean::Constraint>(&mut self) -> Result<bool, Self::Error> {
        self.buffer.read_bit()
    }
}
