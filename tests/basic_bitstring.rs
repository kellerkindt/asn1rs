#![recursion_limit = "512"]

mod test_utils;

use asn1rs::descriptor::bitstring::BitVec;
use test_utils::*;

asn_to_rust!(
    r"BasicBitString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Unconstrained ::= SEQUENCE {
        abc BIT STRING
    }
    
    BasicConstrained ::= SEQUENCE {
        abc BIT STRING (SIZE(8))
    }
    
    BasicConstrainedSmall ::= SEQUENCE {
        abc BIT STRING (SIZE(4..6))
    }
    
    BasicConstrainedExtensible ::= SEQUENCE {
        abc BIT STRING (SIZE(4..6,...))
    }
    
    SomeContainer ::= SEQUENCE {
        some-value BIT STRING {
            very-important-flag  (0),
            not-so-important-flag(1)
        } (SIZE(2))
    }
    
    END"
);

#[test]
fn test_some_container_flag_set() {
    let mut c = SomeContainer {
        some_value: BitVec::with_len(2),
    };
    c.some_value
        .set_bit(SomeContainer::SOME_VALUE_VERY_IMPORTANT_FLAG);
    serialize_and_deserialize_uper(2, &[0x80], &c);
}

#[test]
fn test_unconstrained_6_bits() {
    // from playground
    serialize_and_deserialize_uper(
        14,
        &[0x06, 0xAC],
        &Unconstrained {
            abc: BitVec::from_bytes(vec![0b1010_1100], 6),
        },
    );
}

#[test]
fn test_unconstrained_5_bytes() {
    // from playground
    serialize_and_deserialize_uper(
        48,
        &[0x28, 0x12, 0x34, 0x56, 0x78, 0x90],
        &Unconstrained {
            abc: BitVec::from_all_bytes(vec![0x12, 0x34, 0x56, 0x78, 0x90]),
        },
    );
}

#[test]
fn test_fixed_size() {
    // from playground
    serialize_and_deserialize_uper(
        8,
        &[0x12],
        &BasicConstrained {
            abc: BitVec::from_all_bytes(vec![0x12]),
        },
    );
}

#[test]
#[should_panic(expected = "SizeNotInRange(8, 4, 6)")]
fn test_too_large() {
    // from playground
    serialize_and_deserialize_uper(
        8,
        &[0x12],
        &BasicConstrainedSmall {
            abc: BitVec::from_all_bytes(vec![0x12]),
        },
    );
}

#[test]
fn test_small_max() {
    // from playground
    serialize_and_deserialize_uper(
        8,
        &[0xBF],
        &BasicConstrainedSmall {
            abc: BitVec::from_bytes(vec![0xff], 6),
        },
    );
}

#[test]
fn test_extensible_small() {
    // from playground
    serialize_and_deserialize_uper(
        9,
        &[0x55, 0x80],
        &BasicConstrainedExtensible {
            abc: BitVec::from_bytes(vec![0xaf], 6),
        },
    );
}

#[test]
fn test_extensible_extended_1() {
    // from playground
    serialize_and_deserialize_uper(
        16,
        &[0x83, 0xD6],
        &BasicConstrainedExtensible {
            abc: BitVec::from_bytes(vec![0b1010_1100], 7),
        },
    );
}

#[test]
fn test_extensible_extended_7() {
    // from playground
    serialize_and_deserialize_uper(
        23,
        &[0x87, 0x56, 0xAC],
        &BasicConstrainedExtensible {
            abc: BitVec::from_bytes(vec![0b1010_1101, 0b0101_1000], 14),
        },
    );
}
