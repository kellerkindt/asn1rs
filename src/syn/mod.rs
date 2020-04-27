use std::marker::PhantomData;

pub mod enumerated;
pub mod io;
pub mod numbers;
pub mod optional;
pub mod sequence;
pub mod utf8string;

pub use enumerated::Enumerated;
pub use numbers::Integer;
pub use sequence::Sequence;
pub use utf8string::Utf8String;

pub trait Reader {
    type Error;

    fn read<T: Readable>(&mut self) -> Result<T, Self::Error>
    where
        Self: Sized,
    {
        T::read(self)
    }

    fn read_sequence<
        C: sequence::Constraint,
        S: Sized,
        F: Fn(&mut Self) -> Result<S, Self::Error>,
    >(
        &mut self,
        f: F,
    ) -> Result<S, Self::Error>;

    fn read_enumerated<C: enumerated::Constraint>(&mut self) -> Result<C, Self::Error>;

    fn read_opt<T: ReadableType>(&mut self) -> Result<Option<T::Type>, Self::Error>;

    fn read_int(&mut self, range: (i64, i64)) -> Result<i64, Self::Error>;

    fn read_int_max(&mut self) -> Result<u64, Self::Error>;

    fn read_utf8string<C: utf8string::Constraint>(&mut self) -> Result<String, Self::Error>;
}

pub trait Readable: Sized {
    fn read<R: Reader>(reader: &mut R) -> Result<Self, R::Error>;
}

pub trait ReadableType {
    type Type: Sized;

    fn read_ref<R: Reader>(&self, reader: &mut R) -> Result<Self::Type, R::Error> {
        Self::read_value(reader)
    }

    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, R::Error>;
}

impl<T: Readable> ReadableType for T {
    type Type = T;

    fn read_value<R: Reader>(reader: &mut R) -> Result<T, R::Error> {
        T::read(reader)
    }
}

pub trait Writer {
    type Error;

    fn write<T: Writable>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        value.write(self)
    }

    fn write_sequence<C: sequence::Constraint, F: Fn(&mut Self) -> Result<(), Self::Error>>(
        &mut self,
        f: F,
    ) -> Result<(), Self::Error>;

    fn write_enumerated<C: enumerated::Constraint>(
        &mut self,
        enumerated: &C,
    ) -> Result<(), Self::Error>;

    fn write_opt<T: WritableType>(&mut self, value: Option<&T::Type>) -> Result<(), Self::Error>;

    fn write_int(&mut self, value: i64, range: (i64, i64)) -> Result<(), Self::Error>;

    fn write_int_max(&mut self, value: u64) -> Result<(), Self::Error>;

    fn write_utf8string<C: utf8string::Constraint>(
        &mut self,
        value: &str,
    ) -> Result<(), Self::Error>;
}

pub trait Writable {
    fn write<W: Writer>(&self, writer: &mut W) -> Result<(), W::Error>;
}

pub trait WritableType {
    type Type;

    fn write_ref<W: Writer>(&self, writer: &mut W, value: &Self::Type) -> Result<(), W::Error> {
        Self::write_value(writer, value)
    }

    fn write_value<W: Writer>(writer: &mut W, value: &Self::Type) -> Result<(), W::Error>;
}

pub struct AsnType<T>(PhantomData<T>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syn::io::PrintlnWriter;
    use crate::syn::sequence::Sequence;
    use crate::syn::utf8string::Utf8String;

    #[test]
    fn test_compilable() {
        struct Whatever {
            name: String,
            opt: Option<String>,
            some: Option<String>,
        }

        type AsnDefWhatever = Sequence<Whatever>;
        type AsnDefWhateverName = Utf8String;
        type AsnDefWhateverOpt = Option<Utf8String>;
        type AsnDefWhateverSome = Option<Utf8String>;

        impl sequence::Constraint for Whatever {
            const NAME: &'static str = "Whatever";
            const OPTIONAL_FIELDS: usize = 2;

            fn read_seq<R: Reader>(reader: &mut R) -> Result<Self, <R as Reader>::Error>
            where
                Self: Sized,
            {
                Ok(Self {
                    name: AsnDefWhateverName::read_value(reader)?,
                    opt: AsnDefWhateverOpt::read_value(reader)?,
                    some: AsnDefWhateverSome::read_value(reader)?,
                })
            }

            fn write_seq<W: Writer>(&self, writer: &mut W) -> Result<(), <W as Writer>::Error> {
                AsnDefWhateverName::write_value(writer, &self.name)?;
                AsnDefWhateverOpt::write_value(writer, &self.opt)?;
                AsnDefWhateverSome::write_value(writer, &self.some)?;
                Ok(())
            }
        }

        impl Writable for Whatever {
            fn write<W: Writer>(&self, writer: &mut W) -> Result<(), <W as Writer>::Error> {
                AsnDefWhatever::write_value(writer, self)
            }
        }

        let mut writer = PrintlnWriter::default();
        let value = Whatever {
            name: "SeGreatName".to_string(),
            opt: None,
            some: Some("Lorem Ipsum".to_string()),
        };

        // Writing sequence Whatever
        //  Writing Utf8String(MIN..MAX): SeGreatName
        //  Writing OPTIONAL
        //   None
        //  Writing OPTIONAL
        //   Some
        //    Writing Utf8String(MIN..MAX): Lorem Ipsum
        //        value.write(&mut writer).unwrap();
        writer.write(&value).unwrap();
    }
}
