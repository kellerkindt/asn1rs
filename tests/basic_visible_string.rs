#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicBitString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Unconstrained ::= SEQUENCE {
        abc VisibleString
    }
    
    BasicConstrained ::= SEQUENCE {
        abc VisibleString (SIZE(8))
    }
    
    BasicConstrainedSmall ::= SEQUENCE {
        abc VisibleString (SIZE(4..6))
    }
    
    BasicConstrainedExtensible ::= SEQUENCE {
        abc VisibleString (SIZE(4..6,...))
    } 
    
    END"
);

#[test]
fn detect_only_invalid_character() {
    let mut writer = asn1rs::syn::io::UperWriter::default();
    let result = Unconstrained {
        abc: " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\u{7F}"
            .to_string(),
    }
    .write(&mut writer);
    assert_eq!(
        Err(asn1rs::io::per::ErrorKind::InvalidString(
            asn1rs::model::Charset::Visible,
            '\u{7F}',
            95
        )
        .into()),
        result
    )
}

#[test]
fn test_unconstrained() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 84 + 1,
        &[
            0x5f, 0x40, 0x85, 0x12, 0x34, 0x89, 0x53, 0x27, 0x50, 0xa5, 0x52, 0xb5, 0x8b, 0x57, 0x2f,
            0x60, 0xc5, 0x93, 0x36, 0x8d, 0x5b, 0x37, 0x70, 0xe5, 0xd3, 0xb7, 0x8f, 0x5f, 0x3f, 0x81,
            0x06, 0x14, 0x38, 0x91, 0x63, 0x47, 0x91, 0x26, 0x54, 0xb9, 0x93, 0x67, 0x4f, 0xa1, 0x46,
            0x95, 0x3a, 0x95, 0x6b, 0x57, 0xb1, 0x66, 0xd5, 0xbb, 0x97, 0x6f, 0x5f, 0xc1, 0x87, 0x16,
            0x3c, 0x99, 0x73, 0x67, 0xd1, 0xa7, 0x56, 0xbd, 0x9b, 0x77, 0x6f, 0xe1, 0xc7, 0x97, 0x3e,
            0x9d, 0x7b, 0x77, 0xf1, 0xe7, 0xd7, 0xbf, 0x9f, 0x7f, 0x00
        ],
        &Unconstrained {
            abc: " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"
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
