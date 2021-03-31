mod test_utils;

use test_utils::*;

asn_to_rust!(
    r"MyDef DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN

    ProtobufSequence ::= SEQUENCE {
        inner           SEQUENCE { magic-number INTEGER } 
    }

    ProtobufSequenceExt ::= SEQUENCE {
        lone-bool       BOOLEAN,
        inner           SEQUENCE { magic-number INTEGER },
        another-string  UTF8String
    }

    END"
);

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence() {
    serialize_and_deserialize_protobuf(
        &[10, 2, 8, 42],
        &ProtobufSequence {
            inner: ProtobufSequenceInner { magic_number: 42 },
        },
    )
}

#[test]
#[cfg(feature = "protobuf")]
fn test_sequence_ext() {
    serialize_and_deserialize_protobuf(
        &[8, 0, 18, 3, 8, 185, 10, 26, 3, 101, 120, 116],
        &ProtobufSequenceExt {
            lone_bool: false,
            inner: ProtobufSequenceExtInner { magic_number: 1337 },
            another_string: "ext".into(),
        },
    )
}
