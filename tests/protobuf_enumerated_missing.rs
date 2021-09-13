mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ComplexType ::= SEQUENCE {
        enum-one ENUMERATED {
            A,
            B,
            C
        },
        enum-two ENUMERATEd {
            AA,
            BB,
            CC
        }
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_missing_enum_one() {
    assert_eq!(
        ComplexType {
            enum_one: ComplexTypeEnumOne::B,
            enum_two: ComplexTypeEnumTwo::Aa,
        },
        deserialize_protobuf(&[8, 1],)
    );
}

#[test]
#[cfg(feature = "protobuf")]
fn test_missing_enum_two() {
    assert_eq!(
        ComplexType {
            enum_one: ComplexTypeEnumOne::A,
            enum_two: ComplexTypeEnumTwo::Bb,
        },
        deserialize_protobuf(&[16, 1],)
    );
}
