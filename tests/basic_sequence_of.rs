#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicSequenceOf DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Unconstrained ::= SEQUENCE OF INTEGER
    
    BasicConstrained ::= SEQUENCE SIZE(3) OF INTEGER
    
    BasicConstrainedSmall ::= SEQUENCE (SIZE(2..3)) OF INTEGER
    
    BasicConstrainedExtensible ::= SEQUENCE SIZE(2..3,...) OF INTEGER
    
    END"
);

#[test]
fn test_unconstrained() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 11,
        &[
            0x05, 0x01, 0x01, 0x01, 0x02, 0x01, 0x03, 0x01, 0x04, 0x01, 0x05,
        ],
        &Unconstrained(vec![1, 2, 3, 4, 5]),
    );
}

#[test]
fn test_fixed_size() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 6,
        &[0x01, 0x01, 0x01, 0x02, 0x01, 0x03],
        &BasicConstrained(vec![1, 2, 3]),
    );
}

#[test]
#[should_panic(expected = "SizeNotInRange(5, 2, 3)")]
fn test_too_large() {
    // from playground
    serialize_and_deserialize_uper(0, &[], &BasicConstrainedSmall(vec![1, 2, 3, 4, 5]));
}

#[test]
fn test_small_min() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 4 + 1,
        &[0x00, 0x80, 0x80, 0x81, 0x00],
        &BasicConstrainedSmall(vec![1, 2]),
    );
}

#[test]
fn test_small_max() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 6 + 1,
        &[0x80, 0x80, 0x80, 0x81, 0x00, 0x81, 0x80],
        &BasicConstrainedSmall(vec![1, 2, 3]),
    );
}

#[test]
fn test_extensible_small() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 6 + 2,
        &[0x40, 0x40, 0x40, 0x40, 0x80, 0x40, 0xC0],
        &BasicConstrainedExtensible(vec![1, 2, 3]),
    );
}

#[test]
fn test_extensible_extended() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 11 + 1,
        &[
            0x82, 0x80, 0x80, 0x80, 0x81, 0x00, 0x81, 0x80, 0x82, 0x00, 0x82, 0x80,
        ],
        &BasicConstrainedExtensible(vec![1, 2, 3, 4, 5]),
    );
}
