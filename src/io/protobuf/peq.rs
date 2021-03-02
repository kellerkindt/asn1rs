use crate::syn::BitVec;

/// In protobuf default-ish-values - such as '0' for numbers - might be serialized as `null`/`None`
/// if this is possible in the current context. [`ProtobufEq`] will consider these values as equal
/// while the strict implementations of [`PartialEq`] and [`Eq`] will consider them as not equal.
///
/// ```rust
/// use asn1rs::io::protobuf::ProtobufEq;
///
/// // behaviour is equal to (Partial-)Eq in non-optional scenarios
/// assert_eq!(0_u64.protobuf_eq(&0_u64), 0_u64.eq(&0));
/// assert_eq!(1_u64.protobuf_eq(&0), 1_u64.eq(&0));
///
/// // behaviour might differ from (Partial-)Eq in optional scenarios
/// assert_ne!(Some(0_u64).protobuf_eq(&None), Some(0_u64).eq(&None));
/// assert_ne!(Some(String::default()).protobuf_eq(&None), Some(String::default()).eq(&None));
///
/// // behaviour might not differ from (Partial-)Eq in optional scenarios
/// assert_eq!(Some(0_u64).protobuf_eq(&Some(0)), Some(0_u64).eq(&Some(0)));
/// assert_eq!(Some(1_u64).protobuf_eq(&None), Some(1_u64).eq(&None));
/// ```
pub trait ProtobufEq<Rhs: ?Sized = Self> {
    /// Checks whether this and the other value are equal for the protobuf protocol, which considers
    /// default-ish values and `null`/`None` as equal.
    fn protobuf_eq(&self, other: &Rhs) -> bool;

    /// Inverse of [`ProtobufEq::protobuf_eq`]
    fn protobuf_ne(&self, other: &Rhs) -> bool {
        !self.protobuf_eq(other)
    }
}

impl<T: ProtobufEq + Default + PartialEq> ProtobufEq<Option<T>> for Option<T> {
    fn protobuf_eq(&self, other: &Option<T>) -> bool {
        match self {
            Some(ref v) => match other {
                Some(ref v_other) => v.protobuf_eq(v_other),
                None => v == &T::default(),
            },
            None => match other {
                Some(ref v_other) => &T::default() == v_other,
                None => true,
            },
        }
    }
}

impl<T: ProtobufEq> ProtobufEq<Vec<T>> for Vec<T> {
    fn protobuf_eq(&self, other: &Vec<T>) -> bool {
        if self.len() == other.len() {
            for (i, v) in self.iter().enumerate() {
                if !other[i].protobuf_eq(v) {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
}

impl ProtobufEq<BitVec> for BitVec {
    fn protobuf_eq(&self, other: &BitVec) -> bool {
        self.eq(other)
    }
}

impl ProtobufEq<bool> for bool {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<u8> for u8 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<u16> for u16 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<u32> for u32 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<u64> for u64 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<i8> for i8 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<i16> for i16 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<i32> for i32 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<i64> for i64 {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl ProtobufEq<String> for String {
    fn protobuf_eq(&self, other: &Self) -> bool {
        self == other
    }
}
