#![recursion_limit = "512"]

mod test_utils;

use asn1rs::model::Tag;
use test_utils::*;

asn_to_rust!(
    r"BasicSet DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

        Basic ::= [5] SET {
            abc [APPLICATION 7] UTF8String,
            def INTEGER
        }
      
        Extensible ::= [5] SET {
            abc [APPLICATION 7] UTF8String,
            def INTEGER,
            ...,
            jkl [APPLICATION 3] UTF8String,
            ghi [APPLICATION 5] UTF8String
        }
        
        ImplicitNoReorder ::= SET {
            abc UTF8String,
            def INTEGER          
        }
          
    END"
);

#[test]
fn test_basic() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 15,
        &[
            // serialization order def -> abc
            0x02, 0x03, 0x0A, 0x0B, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x77, 0x6F, 0x72, 0x6C, 0x64,
        ],
        &Basic {
            def: 778,
            abc: "hello world".to_string(),
        },
    );
}

#[test]
fn test_extensible() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 22 + 2,
        &[
            // serialization order def -> abc -> jkl -> ghi
            0x81, 0x01, 0x83, 0x03, 0xB1, 0x3C, 0xB2, 0x90, 0x31, 0x3C, 0xB2, 0x81, 0xC1, 0x00,
            0xDA, 0x9A, 0xDB, 0x01, 0x00, 0xD9, 0xDA, 0x1A, 0x40,
        ],
        &Extensible {
            def: 774,
            abc: "bye bye".to_string(),
            jkl: Some("jkl".to_string()),
            ghi: Some("ghi".to_string()),
        },
    );
}

#[test]
fn test_extensible_2() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 11 + 1,
        &[
            // serialization order def -> abc -> jkl -> ghi
            0x01, 0x01, 0x83, 0x03, 0xB1, 0x3C, 0xB2, 0x90, 0x31, 0x3C, 0xB2, 0x80,
        ],
        &Extensible {
            def: 774,
            abc: "bye bye".to_string(),
            jkl: None,
            ghi: None,
        },
    );
}

/// ```asn
/// Extensible ::= {
///     abc "bye bye",
///     def 774,
///     jkl "jkl"
/// }
/// ```
#[test]
fn test_extensible_4() {
    // from playground
    serialize_and_deserialize_uper(
        8 * 17 + 2,
        &[
            0x81, 0x01, 0x83, 0x03, 0xB1, 0x3C, 0xB2, 0x90, 0x31, 0x3C, 0xB2, 0x81, 0x81, 0x00,
            0xDA, 0x9A, 0xDB, 0x00,
        ],
        &Extensible {
            def: 774,
            abc: "bye bye".to_string(),
            jkl: Some("jkl".to_string()),
            ghi: None,
        },
    );
}

#[test]
fn test_implicit_tag_assignment() {
    use asn1rs::syn::common::Constraint;

    // implicit tagging, therefore no reordering
    assert_eq!(
        Tag::ContextSpecific(0),
        ___asn1rs_ImplicitNoReorderFieldAbcConstraint::TAG
    );
    assert_eq!(
        Tag::ContextSpecific(1),
        ___asn1rs_ImplicitNoReorderFieldDefConstraint::TAG
    );
}

#[test]
fn test_implicit_no_reorder() {
    serialize_and_deserialize_uper(
        8 * 8,
        &[0x05, 0x62, 0x65, 0x72, 0x6E, 0x64, 0x01, 0x37],
        &ImplicitNoReorder {
            abc: "bernd".to_string(),
            def: 55,
        },
    )
}
