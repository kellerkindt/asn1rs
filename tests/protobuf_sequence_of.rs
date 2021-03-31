mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufSequenceOf ::= SEQUENCE {
        many-sint32     SEQUENCE OF INTEGER (-2147483648..2147483647)
    }

    ProtobufSequenceOfExt ::= SEQUENCE {
        lone-bool       BOOLEAN,
        many-sint32     SEQUENCE OF INTEGER (-2147483648..2147483647),
        another-string  UTF8String
    }

    ProtobufSequenceOfExtOpt ::= SEQUENCE {
        lone-bool       BOOLEAN,
        many-sint32     SEQUENCE OF INTEGER (-2147483648..2147483647) OPTIONAL,
        another-string  UTF8String
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_of_empty() {
    serialize_and_deserialize_protobuf(
        &[],
        &ProtobufSequenceOf {
            many_sint32: Vec::default(),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_of_empty_ext() {
    serialize_and_deserialize_protobuf(
        &[8, 1, 26, 5, 101, 109, 112, 116, 121],
        &ProtobufSequenceOfExt {
            lone_bool: true,
            many_sint32: Vec::default(),
            another_string: "empty".into(),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_of_single() {
    serialize_and_deserialize_protobuf(
        &[8, 1],
        &ProtobufSequenceOf {
            many_sint32: vec![-1_i32],
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_of_single_ext() {
    serialize_and_deserialize_protobuf(
        &[8, 0, 16, 1, 26, 6, 115, 105, 110, 103, 108, 101],
        &ProtobufSequenceOfExt {
            lone_bool: false,
            many_sint32: vec![-1_i32],
            another_string: "single".into(),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_of_multiple() {
    serialize_and_deserialize_protobuf(
        &[8, 1, 8, 4, 8, 6, 8, 8, 8, 128, 16, 8, 255, 143, 226, 9],
        &ProtobufSequenceOf {
            many_sint32: vec![-1_i32, 2, 3, 4, 1024, -1024_1024],
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_of_multiple_ext() {
    serialize_and_deserialize_protobuf(
        &[
            8, 0, 16, 1, 16, 4, 16, 6, 16, 8, 16, 128, 16, 16, 255, 143, 226, 9, 26, 8, 109, 117,
            108, 116, 105, 112, 108, 101,
        ],
        &ProtobufSequenceOfExt {
            lone_bool: false,
            many_sint32: vec![-1_i32, 2, 3, 4, 1024, -1024_1024],
            another_string: "multiple".into(),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_of_multiple_ext_opt_some() {
    serialize_and_deserialize_protobuf(
        &[
            8, 0, 16, 1, 16, 4, 16, 6, 16, 8, 16, 128, 16, 16, 255, 143, 226, 9, 26, 8, 109, 117,
            108, 116, 105, 112, 108, 101,
        ],
        &ProtobufSequenceOfExtOpt {
            lone_bool: false,
            many_sint32: Some(vec![-1_i32, 2, 3, 4, 1024, -1024_1024]),
            another_string: "multiple".into(),
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_of_multiple_ext_opt_none() {
    serialize_and_deserialize_protobuf(
        &[8, 0, 26, 8, 109, 117, 108, 116, 105, 112, 108, 101],
        &ProtobufSequenceOfExtOpt {
            lone_bool: false,
            many_sint32: None,
            another_string: "multiple".into(),
        },
    )
}
