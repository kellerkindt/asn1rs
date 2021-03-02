use crate::syn::BitVec;

pub trait ProtobufEq<Rhs: ?Sized = Self> {
    fn protobuf_eq(&self, other: &Rhs) -> bool;
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
