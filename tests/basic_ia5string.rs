#![recursion_limit = "512"]

mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"BasicBitString DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Unconstrained ::= SEQUENCE {
        abc IA5String
    }
    
    BasicConstrained ::= SEQUENCE {
        abc IA5String (SIZE(8))
    }
    
    BasicConstrainedSmall ::= SEQUENCE {
        abc IA5String (SIZE(4..6))
    }
    
    BasicConstrainedExtensible ::= SEQUENCE {
        abc IA5String (SIZE(4..6,...))
    } 
    
    END"
);

#[test]
fn detect_only_invalid_character() {
    let mut writer = asn1rs::syn::io::UperWriter::default();
    let result = Unconstrained {
        abc: "\u{00}\u{01}\u{02}\u{03}\u{04}\u{05}\u{06}\u{07}\u{08}\u{09}\u{0A}\u{0B}\u{0C}\u{0D}\u{0E}\u{0F}\u{10}\u{11}\u{12}\u{13}\u{14}\u{15}\u{16}\u{17}\u{18}\u{19}\u{1A}\u{1B}\u{1C}\u{1D}\u{1E}\u{1F} !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\u{7F}\u{80}"
            .to_string(),
    }
        .write(&mut writer);
    assert_eq!(
        Err(asn1rs::io::per::Error::InvalidString(
            asn1rs::model::Charset::Ia5,
            '\u{80}',
            128
        )),
        result
    )
}

#[test]
fn test_unconstrained() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 12 + 3,
        &[
            0x0D, 0xEB, 0xBB, 0x1E, 0xFD, 0xDC, 0xFA, 0x72, 0xC3, 0xA7, 0x76, 0x5C, 0x80,
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
        8 * 7,
        &[0xCB, 0xE3, 0x0E, 0x3E, 0x9B, 0x3C, 0xB8],
        &BasicConstrained {
            abc: "exactly8".to_string(),
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
        8 * 3 + 6,
        &[0x33, 0x6F, 0xEB, 0xC8],
        &BasicConstrainedSmall {
            abc: "four".to_string(),
        },
    );
}

#[test]
fn test_small_max() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 5 + 4,
        &[0xB9, 0xAD, 0xD2, 0xB7, 0xC2, 0x10],
        &BasicConstrainedSmall {
            abc: "s-i-x!".to_string(),
        },
    );
}

#[test]
fn test_extensible_small() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 3 + 7,
        &[0x19, 0xB7, 0xF5, 0xE4],
        &BasicConstrainedExtensible {
            abc: "four".to_string(),
        },
    );
}

#[test]
fn test_extensible_extended() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 7 + 2,
        &[0x83, 0xF3, 0xCB, 0xDB, 0x2E, 0xE4, 0x28, 0x40],
        &BasicConstrainedExtensible {
            abc: "seven!!".to_string(),
        },
    );
}
