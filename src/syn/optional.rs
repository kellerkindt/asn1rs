use crate::syn::{ReadableType, Reader, WritableType, Writer};

impl<T: WritableType> WritableType for Option<T> {
    type Type = Option<T::Type>;

    fn write_value<W: Writer>(
        writer: &mut W,
        value: &Self::Type,
    ) -> Result<(), <W as Writer>::Error> {
        writer.write_opt::<T>(value.as_ref())
    }
}

impl<T: ReadableType> ReadableType for Option<T> {
    type Type = Option<T::Type>;

    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        reader.read_opt::<T>()
    }
}
