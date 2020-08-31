//! This module contains defines traits to encode and decode basic ASN.1 primitives and types of
//! which the encoding/decoding depends on the UNALIGNED flag.
//! The idea is to provide all building blocks to composite the more complex types on top of the
//! traits without caring about the representation being ALIGNED or UNALIGNED.

pub mod err;
pub mod unaligned;

pub use err::Error;

pub trait PackedRead {
    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 12, the boolean type is represented
    /// through a single bit, where 1 represents `true` and 0 represents `false`.
    fn read_boolean(&mut self) -> Result<bool, Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.7, value that can be a negative,
    /// zero or positive whole number and has no lower- or upper-bound constraints
    fn read_2s_compliment_binary_integer(&mut self, bit_len: u64) -> Result<i64, Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.7, a constrained whole number
    /// is a whole number with a lower- and upper-bound constrained
    fn read_constrained_whole_number(
        &mut self,
        lower_bound: i64,
        upper_bound: i64,
    ) -> Result<i64, Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.17, the length determinant is
    /// a number used to count bits, octets (bytes), characters or components
    fn read_length_determinant(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
    ) -> Result<u64, Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.19, a number without constrains
    /// and is likely to be small. It is used where small lengths are more likely than large values.
    fn read_normally_small_length(&mut self) -> Result<u64, Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.18, an unconstrained integer
    /// where small numbers appear more often the large numbers.
    fn read_normally_small_non_negative_whole_number(&mut self) -> Result<u64, Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.20,
    fn read_non_negative_binary_integer(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
    ) -> Result<u64, Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.24, a semi constrained whole
    /// number is a whole number with a lower-bound constrained but no upper-bound constrained
    fn read_semi_constrained_whole_number(&mut self, lower_bound: i64) -> Result<i64, Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.27, a semi constrained whole
    /// number is a whole number with a lower-bound constrained but no upper-bound constrained
    fn read_unconstrained_whole_number(&mut self) -> Result<i64, Error>;

    fn read_bitstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
    ) -> Result<(Vec<u8>, u64), Error>;

    fn read_octetstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
    ) -> Result<Vec<u8>, Error>;

    fn read_choice_index(&mut self, std_variants: u64, extensible: bool) -> Result<u64, Error>;

    fn read_enumeration_index(&mut self, std_variants: u64, extensible: bool)
        -> Result<u64, Error>;
}

pub trait PackedWrite {
    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 12, the boolean type is represented
    /// through a single bit, where 1 represents `true` and 0 represents `false`.
    fn write_boolean(&mut self, boolean: bool) -> Result<(), Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.7, value that can be a negative,
    /// zero or positive whole number and has no lower- or upper-bound constraints
    fn write_2s_compliment_binary_integer(&mut self, bit_len: u64, value: i64)
        -> Result<(), Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.7, a constrained whole number
    /// is a whole number with a lower- and upper-bound constrained
    fn write_constrained_whole_number(
        &mut self,
        lower_bound: i64,
        upper_bound: i64,
        value: i64,
    ) -> Result<(), Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.17, the length determinant is
    /// a number used to count bits, octets (bytes), characters or components
    fn write_length_determinant(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
        length: u64,
    ) -> Result<(), Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.19, a number without constrains
    /// and is likely to be small. It is used where small lengths are more likely than large values.
    fn write_normally_small_length(&mut self, value: u64) -> Result<(), Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.18, an unconstrained integer
    /// where small numbers appear more often the large numbers.
    fn write_normally_small_non_negative_whole_number(&mut self, value: u64) -> Result<(), Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.20,
    fn write_non_negative_binary_integer(
        &mut self,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
        value: u64,
    ) -> Result<(), Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.24, a semi constrained whole
    /// number is a whole number with a lower-bound constrained but no upper-bound constrained
    fn write_semi_constrained_whole_number(
        &mut self,
        lower_bound: i64,
        value: i64,
    ) -> Result<(), Error>;

    /// According to ITU-TX.691 | ISO/IEC 8825-2:2015, chapter 3.7.27, a semi constrained whole
    /// number is a whole number with a lower-bound constrained but no upper-bound constrained
    fn write_unconstrained_whole_number(&mut self, value: i64) -> Result<(), Error>;

    fn write_bitstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
        src: &[u8],
        offset: u64,
        len: u64,
    ) -> Result<(), Error>;

    fn write_octetstring(
        &mut self,
        lower_bound_size: Option<u64>,
        upper_bound_size: Option<u64>,
        extensible: bool,
        src: &[u8],
    ) -> Result<(), Error>;

    fn write_choice_index(
        &mut self,
        std_variants: u64,
        extensible: bool,
        index: u64,
    ) -> Result<(), Error>;

    fn write_enumeration_index(
        &mut self,
        std_variants: u64,
        extensible: bool,
        index: u64,
    ) -> Result<(), Error>;
}
