mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufOptionals ::= SEQUENCE {
        optional-bool       BOOLEAN OPTIONAL,
        optional-utf8string UTF8String OPTIONAL,
        optional-sint32     INTEGER (-2147483648..2147483647) OPTIONAL
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_optionals_present() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[8, 1, 18, 6, 115, 116, 114, 105, 110, 103, 24, 84],
        &ProtobufOptionals {
            optional_bool: Some(true),
            optional_utf8string: Some("string".into()),
            optional_sint32: Some(42),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_optionals_absent() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[],
        &ProtobufOptionals {
            optional_bool: None,
            optional_utf8string: None,
            optional_sint32: None,
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_optionals_mixed() {
    serialize_and_deserialize_protobuf(
        // data is from the output of the legacy serializer
        &[8, 0, 24, 242, 20],
        &ProtobufOptionals {
            optional_bool: Some(false),
            optional_utf8string: None,
            optional_sint32: Some(1337_i32),
        },
    )
}
