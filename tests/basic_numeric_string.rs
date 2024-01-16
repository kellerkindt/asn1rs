#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicBitString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Unconstrained ::= SEQUENCE {
        abc NumericString
    }
    
    BasicConstrained ::= SEQUENCE {
        abc NumericString (SIZE(8))
    }
    
    BasicConstrainedSmall ::= SEQUENCE {
        abc NumericString (SIZE(4..6))
    }
    
    BasicConstrainedExtensible ::= SEQUENCE {
        abc NumericString (SIZE(4..6,...))
    } 
    
    END"
);

#[test]
fn detect_only_invalid_character() {
    let mut writer = asn1rs::syn::io::UperWriter::default();
    let result = Unconstrained {
        abc: " 0123456789x".to_string(),
    }
    .write(&mut writer);
    assert_eq!(
        Err(asn1rs::io::per::ErrorKind::InvalidString(
            asn1rs::model::asn::Charset::Numeric,
            'x',
            11
        )
        .into()),
        result
    )
}

#[test]
fn test_unconstrained() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 6 + 4,
        &[0x0B, 0x01, 0x23, 0x45, 0x67, 0x89, 0xA0],
        &Unconstrained {
            abc: " 0123456789".to_string(),
        },
    );
}

#[test]
fn test_fixed_size() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 4,
        &[0x23, 0x45, 0x67, 0x89],
        &BasicConstrained {
            abc: "12345678".to_string(),
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
            abc: "12345678".to_string(),
        },
    );
}

#[test]
fn test_small_min() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 2 + 2,
        &[0x08, 0xD1, 0x40],
        &BasicConstrainedSmall {
            abc: "1234".to_string(),
        },
    );
}

#[test]
fn test_small_max() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 3 + 2,
        &[0x88, 0xD1, 0x59, 0xC0],
        &BasicConstrainedSmall {
            abc: "123456".to_string(),
        },
    );
}

#[test]
fn test_extensible_small() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 2 + 3,
        &[0x04, 0x68, 0xA0],
        &BasicConstrainedExtensible {
            abc: "1234".to_string(),
        },
    );
}

#[test]
fn test_extensible_extended() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 4 + 5,
        &[0x83, 0x91, 0xA2, 0xB3, 0xC0],
        &BasicConstrainedExtensible {
            abc: "1234567".to_string(),
        },
    );
}
