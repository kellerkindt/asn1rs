use crate::io::per::unaligned::BYTE_LEN;
use crate::syn::{ReadableType, Reader, WritableType, Writer};
use std::marker::PhantomData;

pub struct BitString<C: Constraint = NoConstraint>(PhantomData<C>);

impl<C: Constraint> Default for BitString<C> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub trait Constraint {
    const MIN: Option<u64> = None;
    const MAX: Option<u64> = None;
    const EXTENSIBLE: bool = false;
}

#[derive(Default)]
pub struct NoConstraint;
impl Constraint for NoConstraint {}

impl<C: Constraint> WritableType for BitString<C> {
    type Type = BitVec;

    #[inline]
    fn write_value<W: Writer>(writer: &mut W, value: &Self::Type) -> Result<(), W::Error> {
        writer.write_bit_string::<C>(value.as_byte_slice(), value.1)
    }
}

impl<C: Constraint> ReadableType for BitString<C> {
    type Type = BitVec;

    #[inline]
    fn read_value<R: Reader>(reader: &mut R) -> Result<Self::Type, <R as Reader>::Error> {
        let (vec, bit_len) = reader.read_bit_string::<C>()?;
        Ok(BitVec(vec, bit_len))
    }
}

#[derive(Debug, Default, Clone, PartialOrd, PartialEq, Hash)]
pub struct BitVec(Vec<u8>, u64);

impl BitVec {
    pub fn from_all_bytes(bytes: Vec<u8>) -> Self {
        let bit_len = (bytes.len() * BYTE_LEN) as u64;
        Self::from_bytes(bytes, bit_len)
    }

    pub fn from_bytes(mut bytes: Vec<u8>, bit_len: u64) -> Self {
        if bytes.len() * BYTE_LEN > bit_len as usize {
            // ensure bits that are zeroed out
            let mask = 0xFF_u8 >> (bit_len as usize % BYTE_LEN);
            let index = bit_len as usize / BYTE_LEN;
            bytes[index] &= !mask;
        } else if bytes.len() * BYTE_LEN < bit_len as usize {
            let missing_bytes = ((bit_len as usize + 7) / 8) - bytes.len();
            bytes.extend(core::iter::repeat(0u8).take(missing_bytes));
        }
        BitVec(bytes, bit_len)
    }

    pub fn with_len(bits: u64) -> Self {
        let bytes = (bits as usize + 7) / 8;
        BitVec(core::iter::repeat(0u8).take(bytes).collect(), bits)
    }

    /// # Panics
    ///
    /// If the given `Vec<u8>` is not at least 4 bytes large
    pub fn from_vec_with_trailing_bit_len(mut bytes: Vec<u8>) -> Self {
        const U64_SIZE: usize = std::mem::size_of::<u64>();
        let bytes_position = bytes.len() - U64_SIZE;
        let mut bit_len_buffer = [0u8; U64_SIZE];
        for i in (0..U64_SIZE).rev() {
            bit_len_buffer[i] = bytes.remove(bytes_position + i);
        }
        Self(bytes, u64::from_be_bytes(bit_len_buffer))
    }

    pub fn to_vec_with_trailing_bit_len(&self) -> Vec<u8> {
        let mut buffer = self.0.clone();
        self.1.to_be_bytes().iter().for_each(|b| buffer.push(*b));
        buffer
    }

    pub fn is_bit_set(&self, bit: u64) -> bool {
        let byte = bit / 8;
        let bit = bit % 8;
        let mask = 0x80_u8 >> bit;
        self.0
            .get(byte as usize)
            .map(|b| *b & mask != 0)
            .unwrap_or(false)
    }

    pub fn set_bit(&mut self, bit: u64) {
        self.ensure_vec_large_enough(bit);
        let byte = bit / 8;
        let bit = bit % 8;
        let mask = 0x80_u8 >> bit;
        self.0[byte as usize] |= mask;
    }

    pub fn reset_bit(&mut self, bit: u64) {
        self.ensure_vec_large_enough(bit);
        let byte = bit / 8;
        let bit = bit % 8;
        let mask = 0x80_u8 >> bit;
        self.0[byte as usize] &= !mask;
    }

    fn ensure_vec_large_enough(&mut self, bits: u64) {
        if bits > self.1 {
            let byte = (bits / 8) as usize;
            for _ in self.0.len()..byte {
                self.0.push(0x00);
            }
            self.1 = bits;
        }
    }

    pub fn bit_len(&self) -> u64 {
        self.1
    }

    pub fn byte_len(&self) -> usize {
        self.0.len()
    }

    pub fn as_byte_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn trailing_bit_len_repr() {
        for bit_len in 0..(BYTE_LEN * 10) {
            for value in 0..u8::MAX {
                let byte_len = (bit_len + 7) / 8;
                let start = BitVec(
                    core::iter::repeat(value).take(byte_len).collect(),
                    bit_len as u64,
                );
                let vec_repr = start.to_vec_with_trailing_bit_len();
                let end = BitVec::from_vec_with_trailing_bit_len(vec_repr);
                assert_eq!(start, end);
            }
        }
    }
}
