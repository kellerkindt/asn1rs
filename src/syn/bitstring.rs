use crate::syn::{ReadableType, Reader, WritableType, Writer};
use std::marker::PhantomData;

pub struct BitString<C: Constraint = NoConstraint>(PhantomData<C>);

impl<C: Constraint> Default for BitString<C> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub trait Constraint {
    const MIN: Option<usize> = None;
    const MAX: Option<usize> = None;
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

#[derive(Debug, Default, Clone, PartialOrd, PartialEq)]
pub struct BitVec(Vec<u8>, usize);

impl BitVec {
    pub fn with_capacity(bits: usize) -> Self {
        let bytes = (bits + 7) / 8;
        BitVec(Vec::with_capacity(bytes), bits)
    }

    pub fn is_bit_set(&self, bit: usize) -> bool {
        let byte = bit / 8;
        let bit = bit % 8;
        let mask = 0x80_u8 >> bit;
        self.0.get(byte).map(|b| *b & mask != 0).unwrap_or(false)
    }

    pub fn set_bit(&mut self, bit: usize) {
        self.ensure_vec_large_enough(bit);
        let byte = bit / 8;
        let bit = bit % 8;
        let mask = 0x80_u8 >> bit;
        self.0[byte] |= mask;
    }

    pub fn reset_bit(&mut self, bit: usize) {
        self.ensure_vec_large_enough(bit);
        let byte = bit / 8;
        let bit = bit % 8;
        let mask = 0x80_u8 >> bit;
        self.0[byte] &= !mask;
    }

    fn ensure_vec_large_enough(&mut self, bits: usize) {
        if bits > self.1 {
            let byte = bits / 8;
            for _ in self.0.len()..byte {
                self.0.push(0x00);
            }
            self.1 = bits;
        }
    }

    pub fn bit_len(&self) -> usize {
        self.1
    }

    pub fn byte_len(&self) -> usize {
        self.0.len()
    }

    pub fn as_byte_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}
