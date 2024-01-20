#![recursion_limit = "512"]
#![cfg(feature = "protobuf")]

use asn1rs::prelude::*;

#[derive(ProtobufEq)]
pub struct SimpleStruct {
    maybe_some_number: Option<u64>,
}

#[test]
pub fn test_none_is_eq_to_zero() {
    assert!(SimpleStruct {
        maybe_some_number: None,
    }
    .protobuf_eq(&SimpleStruct {
        maybe_some_number: Some(0),
    }))
}

#[test]
pub fn test_none_is_non_eq_to_one() {
    assert!(!SimpleStruct {
        maybe_some_number: None,
    }
    .protobuf_eq(&SimpleStruct {
        maybe_some_number: Some(1),
    }))
}

#[test]
pub fn test_one_is_eq_to_one() {
    assert!(SimpleStruct {
        maybe_some_number: Some(1),
    }
    .protobuf_eq(&SimpleStruct {
        maybe_some_number: Some(1),
    }))
}

#[test]
pub fn test_two_is_non_eq_to_one() {
    assert!(!SimpleStruct {
        maybe_some_number: Some(2),
    }
    .protobuf_eq(&SimpleStruct {
        maybe_some_number: Some(1),
    }))
}

#[derive(ProtobufEq)]
pub struct TupleStruct(Option<u64>);

#[test]
pub fn test_tuple_struct() {
    assert!(TupleStruct(None).protobuf_eq(&TupleStruct(Some(0))));
    assert!(TupleStruct(Some(0)).protobuf_eq(&TupleStruct(Some(0))));
    assert!(!TupleStruct(Some(1)).protobuf_eq(&TupleStruct(Some(0))));
}

#[derive(ProtobufEq)]
pub enum DataEnum {
    Abc(u64),
    Def(Option<u64>),
    Ghi(TupleStruct),
}

#[test]
pub fn test_data_enum() {
    assert!(DataEnum::Def(None).protobuf_eq(&DataEnum::Def(Some(0))));
    assert!(DataEnum::Def(Some(0)).protobuf_eq(&DataEnum::Def(Some(0))));
    assert!(!DataEnum::Def(Some(1)).protobuf_eq(&DataEnum::Def(Some(0))));
    assert!(!DataEnum::Abc(1).protobuf_eq(&DataEnum::Ghi(TupleStruct(None))));
}

#[derive(ProtobufEq)]
pub enum SimpleEnum {
    Abc,
    Def,
    Ghi,
}

#[test]
pub fn test_simple_enum() {
    assert!(SimpleEnum::Abc.protobuf_eq(&SimpleEnum::Abc));
    assert!(SimpleEnum::Def.protobuf_eq(&SimpleEnum::Def));
    assert!(SimpleEnum::Ghi.protobuf_eq(&SimpleEnum::Ghi));
    assert!(!SimpleEnum::Abc.protobuf_eq(&SimpleEnum::Ghi));
}
