use crate::descriptor::numbers::Number;
use crate::descriptor::sequence::Constraint;
use crate::descriptor::{Null, ReadableType, Reader, WritableType, Writer};
use crate::protocol::basic::Error;
use crate::protocol::basic::{BasicRead, BasicWrite};
use asn1rs_model::asn::Tag;

pub struct BasicWriter<W: BasicWrite> {
    write: W,
}

impl<W: BasicWrite> From<W> for BasicWriter<W> {
    #[inline]
    fn from(write: W) -> Self {
        Self { write }
    }
}

impl<W: BasicWrite> BasicWriter<W> {
    #[inline]
    pub fn into_inner(self) -> W {
        self.write
    }
}

impl<W: BasicWrite> Writer for BasicWriter<W> {
    type Error = Error;

    fn write_sequence<C: Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        _f: F,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_sequence_of<C: crate::descriptor::sequenceof::Constraint, T: WritableType>(
        &mut self,
        _slice: &[T::Type],
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_set<C: Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        _f: F,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_set_of<C: crate::descriptor::sequenceof::Constraint, T: WritableType>(
        &mut self,
        _slice: &[T::Type],
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_enumerated<C: crate::descriptor::enumerated::Constraint>(
        &mut self,
        _enumerated: &C,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_choice<C: crate::descriptor::choice::Constraint>(
        &mut self,
        _choice: &C,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_opt<T: WritableType>(&mut self, _value: Option<&T::Type>) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_default<
        C: crate::descriptor::default::Constraint<Owned = T::Type>,
        T: WritableType,
    >(
        &mut self,
        _value: &T::Type,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_number<T: Number, C: crate::descriptor::numbers::Constraint<T>>(
        &mut self,
        _value: T,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_utf8string<C: crate::descriptor::utf8string::Constraint>(
        &mut self,
        _value: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_ia5string<C: crate::descriptor::ia5string::Constraint>(
        &mut self,
        _value: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_numeric_string<C: crate::descriptor::numericstring::Constraint>(
        &mut self,
        _value: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_visible_string<C: crate::descriptor::visiblestring::Constraint>(
        &mut self,
        _value: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_printable_string<C: crate::descriptor::printablestring::Constraint>(
        &mut self,
        _value: &str,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_octet_string<C: crate::descriptor::octetstring::Constraint>(
        &mut self,
        _value: &[u8],
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_bit_string<C: crate::descriptor::bitstring::Constraint>(
        &mut self,
        _value: &[u8],
        _bit_len: u64,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_boolean<C: crate::descriptor::boolean::Constraint>(
        &mut self,
        value: bool,
    ) -> Result<(), Self::Error> {
        self.write.write_identifier(C::TAG)?;
        self.write.write_length(1)?;
        self.write.write_boolean(value)?;
        Ok(())
    }

    fn write_null<C: crate::descriptor::null::Constraint>(
        &mut self,
        _value: &Null,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

pub struct BasicReader<R: BasicRead> {
    read: R,
}

impl<W: BasicRead> From<W> for BasicReader<W> {
    #[inline]
    fn from(read: W) -> Self {
        Self { read }
    }
}

impl<W: BasicRead> BasicReader<W> {
    #[inline]
    pub fn into_inner(self) -> W {
        self.read
    }
}

impl<R: BasicRead> Reader for BasicReader<R> {
    type Error = Error;

    fn read_sequence<C: Constraint, S: Sized, F: Fn(&mut Self) -> Result<S, Self::Error>>(
        &mut self,
        _f: F,
    ) -> Result<S, Self::Error> {
        todo!()
    }

    fn read_sequence_of<C: crate::descriptor::sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<T::Type>, Self::Error> {
        todo!()
    }

    fn read_set<C: Constraint, S: Sized, F: Fn(&mut Self) -> Result<S, Self::Error>>(
        &mut self,
        _f: F,
    ) -> Result<S, Self::Error> {
        todo!()
    }

    fn read_set_of<C: crate::descriptor::sequenceof::Constraint, T: ReadableType>(
        &mut self,
    ) -> Result<Vec<T::Type>, Self::Error> {
        todo!()
    }

    fn read_enumerated<C: crate::descriptor::enumerated::Constraint>(
        &mut self,
    ) -> Result<C, Self::Error> {
        todo!()
    }

    fn read_choice<C: crate::descriptor::choice::Constraint>(&mut self) -> Result<C, Self::Error> {
        todo!()
    }

    fn read_opt<T: ReadableType>(&mut self) -> Result<Option<T::Type>, Self::Error> {
        todo!()
    }

    fn read_default<C: crate::descriptor::default::Constraint<Owned = T::Type>, T: ReadableType>(
        &mut self,
    ) -> Result<T::Type, Self::Error> {
        todo!()
    }

    fn read_number<T: Number, C: crate::descriptor::numbers::Constraint<T>>(
        &mut self,
    ) -> Result<T, Self::Error> {
        todo!()
    }

    fn read_utf8string<C: crate::descriptor::utf8string::Constraint>(
        &mut self,
    ) -> Result<String, Self::Error> {
        todo!()
    }

    fn read_ia5string<C: crate::descriptor::ia5string::Constraint>(
        &mut self,
    ) -> Result<String, Self::Error> {
        todo!()
    }

    fn read_numeric_string<C: crate::descriptor::numericstring::Constraint>(
        &mut self,
    ) -> Result<String, Self::Error> {
        todo!()
    }

    fn read_visible_string<C: crate::descriptor::visiblestring::Constraint>(
        &mut self,
    ) -> Result<String, Self::Error> {
        todo!()
    }

    fn read_printable_string<C: crate::descriptor::printablestring::Constraint>(
        &mut self,
    ) -> Result<String, Self::Error> {
        todo!()
    }

    fn read_octet_string<C: crate::descriptor::octetstring::Constraint>(
        &mut self,
    ) -> Result<Vec<u8>, Self::Error> {
        todo!()
    }

    fn read_bit_string<C: crate::descriptor::bitstring::Constraint>(
        &mut self,
    ) -> Result<(Vec<u8>, u64), Self::Error> {
        todo!()
    }

    fn read_boolean<C: crate::descriptor::boolean::Constraint>(
        &mut self,
    ) -> Result<bool, Self::Error> {
        let identifier = self.read.read_identifier()?;
        if identifier.value() != Tag::DEFAULT_BOOLEAN.value() {
            return Err(Error::unexpected_tag(Tag::DEFAULT_BOOLEAN, identifier));
        }
        let expecting = 1_usize..2_usize;
        let length = self.read.read_length()?;
        if !expecting.contains(&length) {
            return Err(Error::unexpected_length(expecting, length));
        }
        self.read.read_boolean()
    }

    fn read_null<C: crate::descriptor::null::Constraint>(&mut self) -> Result<Null, Self::Error> {
        todo!()
    }
}
