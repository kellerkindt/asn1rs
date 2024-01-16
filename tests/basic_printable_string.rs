#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicBitString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Unconstrained ::= SEQUENCE {
        abc PrintableString
    }
    
    BasicConstrained ::= SEQUENCE {
        abc PrintableString (SIZE(8))
    }
    
    BasicConstrainedSmall ::= SEQUENCE {
        abc PrintableString (SIZE(4..6))
    }
    
    BasicConstrainedExtensible ::= SEQUENCE {
        abc PrintableString (SIZE(4..6,...))
    } 
    
    END"
);

#[test]
fn detect_only_invalid_character() {
    let mut writer = asn1rs::syn::io::UperWriter::default();
    let result = Unconstrained {
        abc: " '()+,-./0123456789:=?ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!"
            .to_string(),
    }
    .write(&mut writer);
    assert_eq!(
        Err(asn1rs::io::per::ErrorKind::InvalidString(
            asn1rs::model::asn::Charset::Printable,
            '!',
            74
        )
        .into()),
        result
    )
}

#[test]
fn test_unconstrained() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 65 + 6,
        &[
            0x4A, 0x40, 0x9D, 0x42, 0x95, 0x6B, 0x16, 0xAE, 0x5E, 0xC1, 0x8B, 0x26, 0x6D, 0x1A,
            0xB6, 0x6E, 0xE1, 0xCB, 0xA7, 0xAF, 0xE0, 0xC2, 0x87, 0x12, 0x2C, 0x68, 0xF2, 0x24,
            0xCA, 0x97, 0x32, 0x6C, 0xE9, 0xF4, 0x28, 0xD2, 0xA7, 0x52, 0xAD, 0x6A, 0xF6, 0x2C,
            0xDA, 0xC3, 0x8B, 0x1E, 0x4C, 0xB9, 0xB3, 0xE8, 0xD3, 0xAB, 0x5E, 0xCD, 0xBB, 0xB7,
            0xF0, 0xE3, 0xCB, 0x9F, 0x4E, 0xBD, 0xBB, 0xF8, 0xF3, 0xE8,
        ],
        &Unconstrained {
            abc: " '()+,-./0123456789:=?ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
                .to_string(),
        },
    );
}

#[test]
fn test_fixed_size() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 7,
        &[0x62, 0xC9, 0x9B, 0x46, 0xAD, 0x9B, 0xB8],
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
        8 * 3 + 6,
        &[0x18, 0xB2, 0x66, 0xD0],
        &BasicConstrainedSmall {
            abc: "1234".to_string(),
        },
    );
}

#[test]
fn test_small_max() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 5 + 4,
        &[0x98, 0xB2, 0x66, 0xD1, 0xAB, 0x60],
        &BasicConstrainedSmall {
            abc: "123456".to_string(),
        },
    );
}

#[test]
fn test_extensible_small() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 3 + 7,
        &[0x0C, 0x59, 0x33, 0x68],
        &BasicConstrainedExtensible {
            abc: "1234".to_string(),
        },
    );
}

#[test]
fn test_extensible_extended() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 7 + 2,
        &[0x83, 0xB1, 0x64, 0xCD, 0xA3, 0x56, 0xCD, 0xC0],
        &BasicConstrainedExtensible {
            abc: "1234567".to_string(),
        },
    );
}
