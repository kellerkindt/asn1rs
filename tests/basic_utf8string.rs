#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicBitString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Unconstrained ::= SEQUENCE {
        abc UTF8String
    }
    
    BasicConstrained ::= SEQUENCE {
        abc UTF8String (SIZE(8))
    }

    BasicConstrainedFixedExtensible ::= SEQUENCE {
        abc UTF8String (SIZE(8,...))
    }
    
    BasicConstrainedSmall ::= SEQUENCE {
        abc UTF8String (SIZE(4..6))
    }
    
    BasicConstrainedExtensible ::= SEQUENCE {
        abc UTF8String (SIZE(4..6,...))
    } 
    
    END"
);

#[test]
fn test_unconstrained() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 14,
        &[
            0x0D, 0x75, 0x6E, 0x63, 0x6F, 0x6E, 0x73, 0x74, 0x72, 0x61, 0x69, 0x6E, 0x65, 0x64,
        ],
        &Unconstrained {
            abc: "unconstrained".to_string(),
        },
    );
}

#[test]
fn test_fixed_size() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 9,
        &[0x08, 0x65, 0x78, 0x61, 0x63, 0x74, 0x6C, 0x79, 0x38],
        &BasicConstrained {
            abc: "exactly8".to_string(),
        },
    );
}

#[test]
fn test_fixed_size_extensible_smaller() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 4,
        &[0x03, 0x6C, 0x74, 0x38],
        &BasicConstrainedFixedExtensible {
            abc: "lt8".to_string(),
        },
    );
}

#[test]
fn test_fixed_size_extensible_exact() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 9,
        &[0x08, 0x65, 0x78, 0x61, 0x63, 0x74, 0x6C, 0x79, 0x38],
        &BasicConstrainedFixedExtensible {
            abc: "exactly8".to_string(),
        },
    );
}

#[test]
fn test_fixed_size_extensible_greater() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 10,
        &[0x09, 0x65, 0x78, 0x61, 0x63, 0x74, 0x6C, 0x79, 0x5F, 0x39],
        &BasicConstrainedFixedExtensible {
            abc: "exactly_9".to_string(),
        },
    );
}

#[test]
#[should_panic(expected = "SizeNotInRange(8, 4, 6)")]
fn test_too_large() {
    // from playground
    serialize_and_deserialize_uper(
        0,
        &[],
        &BasicConstrainedSmall {
            abc: "exactly8".to_string(),
        },
    );
}

#[test]
fn test_small_min() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 5,
        &[0x04, 0x66, 0x6F, 0x75, 0x72],
        &BasicConstrainedSmall {
            abc: "four".to_string(),
        },
    );
}

#[test]
fn test_small_max() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 7,
        &[0x06, 0x73, 0x2D, 0x69, 0x2D, 0x78, 0x21],
        &BasicConstrainedSmall {
            abc: "s-i-x!".to_string(),
        },
    );
}

#[test]
fn test_extensible_small() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 5,
        &[0x04, 0x66, 0x6F, 0x75, 0x72],
        &BasicConstrainedExtensible {
            abc: "four".to_string(),
        },
    );
}

#[test]
fn test_extensible_extended() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 8,
        &[0x07, 0x73, 0x65, 0x76, 0x65, 0x6E, 0x21, 0x21],
        &BasicConstrainedExtensible {
            abc: "seven!!".to_string(),
        },
    );
}

#[test]
fn test_single_umlaut() {
    serialize_and_deserialize_uper(
        3 * 8,
        &[0x02, 0xC3, 0xA4],
        &Unconstrained {
            abc: "ä".to_string(),
        },
    )
}

#[test]
fn test_multiple_umlauts() {
    serialize_and_deserialize_uper(
        15 * 8,
        &[
            0x0E, 0xC3, 0xA4, 0xC3, 0xB6, 0xC3, 0xBC, 0xC3, 0x84, 0xC3, 0x96, 0xC3, 0x9C, 0xC3,
            0x9F,
        ],
        &Unconstrained {
            abc: "äöüÄÖÜß".to_string(),
        },
    )
}
